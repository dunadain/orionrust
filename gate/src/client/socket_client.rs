use std::{
    collections::HashMap,
    sync::{atomic::AtomicU8, Arc, Mutex, OnceLock},
    time::Duration,
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use orion::{appinfo, nats_msg, SocketHandle};
use tokio::{select, sync::mpsc, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::{
    config::{protocols, server_config},
    global::nats,
    protocol::{message, packet},
};

use super::{ClientManager, NetClient};

const WAIT_FOR_HANDSHAKE: u8 = 0;
const WAIT_FOR_HANDSHAKE_ACK: u8 = 1;
const READY: u8 = 2;

const HEARTBEAT_INTERVAL: u8 = 30;

enum ErrorCode {
    InvalidHandshake = 1,
    OutedClient = 2,
    InvalidUID = 3,
}

fn check_client(client_ver: u32) -> bool {
    true
}

#[derive(Clone)]
pub struct Client<T: SocketHandle + Sync + Send + Clone + 'static> {
    uid: OnceLock<String>,
    socket: T,
    state: Arc<AtomicU8>,
    heartbeat_recved: mpsc::Sender<()>,
    dead: CancellationToken,
    /// 记录此客户端所连接的每种类型的有状态服务器
    server_map: Arc<Mutex<HashMap<u8, String>>>,
}

// TODO: 从mongodb中加载用户数据到redis（从专门的redis管理服务器加载？）
impl<T: SocketHandle + Sync + Send + Clone + 'static> NetClient for Client<T> {
    type ClientMgrType = ClientManager<Client<T>>;
    async fn receive_msg(self: &Arc<Self>, packet: Bytes, mgr: ClientManager<Client<T>>) {
        let (packet_type, mut decoded_body) = packet::decode(packet);
        match packet_type {
            packet::PacketType::Handshake => {
                if self.state.load(std::sync::atomic::Ordering::SeqCst) != WAIT_FOR_HANDSHAKE {
                    return;
                }
                if decoded_body.len() < 1 || decoded_body.len() < (decoded_body[0] + 5).into() {
                    self.report_error(ErrorCode::InvalidHandshake as u16, "invalid handshake")
                        .await;
                    self.socket.close().await;
                    return;
                }
                let uid_len = decoded_body.get_u8();
                let uid_bytes = decoded_body.slice(..uid_len as usize);
                decoded_body.advance(uid_len as usize);

                let client_ver = decoded_body.get_u32();

                if cfg!(not(debug_assertions)) {
                    if !check_client(client_ver) {
                        self.report_error(ErrorCode::OutedClient as u16, "outdated client")
                            .await;
                        self.socket.close().await;
                        return;
                    }
                }

                let uid = String::from_utf8(uid_bytes.to_vec());
                match uid {
                    Ok(uid) if uid != "" => {
                        // TODO: 剔除重复登录用户
                        let _ = self.uid.set(uid.clone());
                        mgr.bind_connection(uid, self.socket.id());
                    }
                    other => {
                        if let Err(e) = other {
                            self.report_error(ErrorCode::InvalidUID as u16, &format!("{}", e))
                                .await;
                            error!("invalid uid: {}", e);
                        } else {
                            self.report_error(ErrorCode::InvalidUID as u16, "empty uid")
                                .await;
                            error!("empty uid");
                        }
                        self.socket.close().await;
                        return;
                    }
                }
                let mut send_bytes = BytesMut::new();
                send_bytes.put_u8(HEARTBEAT_INTERVAL); // heartbeat interval
                let packet = packet::encode(packet::PacketType::Handshake, send_bytes.freeze());
                self.state
                    .store(WAIT_FOR_HANDSHAKE_ACK, std::sync::atomic::Ordering::SeqCst);
                self.socket.send(packet).await;
            }
            packet::PacketType::HandshakeAck => {
                if self.state.load(std::sync::atomic::Ordering::SeqCst) != WAIT_FOR_HANDSHAKE_ACK {
                    return;
                }
                self.state.store(READY, std::sync::atomic::Ordering::SeqCst);
            }
            packet::PacketType::Heartbeat => {
                let _ = self.heartbeat_recved.send(()).await;
                let packet = packet::encode(packet::PacketType::Heartbeat, Bytes::new());
                self.socket.send(packet).await;
            }
            packet::PacketType::Data => {
                if self.state.load(std::sync::atomic::Ordering::SeqCst) != READY {
                    return;
                }
                let (msg_type, proto_id, reqid, data) = message::decode(decoded_body);
                let proto_str = &protocols()[proto_id as usize];
                let index = proto_str.find("-").expect("should have - in the protocol");
                let server_type = &proto_str[..index];
                let mut subject = "handler.".to_string();
                if server_config()[server_type]["stateless"] == true {
                    subject.push_str(server_type);
                } else {
                    // TODO: add specific server uuid to the subject(eg. handler.servertype/uuid) 要是这个uuid服务器挂了咋办
                };
                let uid = self.uid.get();
                if let None = uid {
                    error!("uid is none");
                    return;
                }
                let payload = nats_msg::encode(
                    self.socket.id(),
                    proto_id,
                    reqid,
                    uid.unwrap(),
                    appinfo().uuid(),
                    data,
                );
                match msg_type {
                    message::MsgType::Request => {
                        let mut reply = appinfo().uuid().to_string();
                        reply.push_str(".reply.");
                        reply.push_str(&reqid.to_string());
                        let result = nats().publish_with_reply(subject, reply, payload).await;
                        if let Err(e) = result {
                            error!("{:?}", e);
                        }
                    }
                    message::MsgType::Notify => {
                        let result = nats().publish(subject.into(), payload).await;
                        if let Err(e) = result {
                            error!("{}", e);
                        }
                    }
                    _ => {}
                };
            }
            _ => {}
        }
    }

    async fn onopen(self: &Arc<Self>) {}

    async fn onclose(self: &Arc<Self>) {
        // TODO: 把此用户相关的数据从缓冲或者其他服务器清理
        self.dead.cancel();
    }

    async fn close(self: &Arc<Self>) {
        self.socket.close().await;
        let token = self.dead.clone();
        token.cancelled().await;
    }

    async fn kick(self: &Arc<Self>) {
        self.socket
            .send(packet::encode(packet::PacketType::Kick, Bytes::new()))
            .await;
    }

    async fn sendmsg(
        self: &Arc<Self>,
        msg_type: message::MsgType,
        proto_id: u16,
        data: Bytes,
        reqid: u8,
    ) {
        if self.state.load(std::sync::atomic::Ordering::SeqCst) != READY {
            return;
        }
        let msgbody = message::encode(msg_type, proto_id, reqid, data);
        let packet = packet::encode(packet::PacketType::Data, msgbody);
        self.socket.send(packet).await;
    }

    async fn report_error(self: &Arc<Self>, error_code: u16, message: &str) {
        let mut msg = BytesMut::new();
        msg.put_u16(error_code);
        // msg.put_u8(message.len() as u8);
        msg.put_slice(message.as_bytes());
        let packet = packet::encode(packet::PacketType::Error, msg.freeze());
        self.socket.send(packet).await;
    }
}

impl<T: SocketHandle + Sync + Send + Clone + 'static> Client<T> {
    pub fn new(socket: T) -> Self {
        let (tx, mut rx) = mpsc::channel(1);

        let s = socket.clone();
        tokio::spawn(async move {
            loop {
                select! {
                        _ = sleep(Duration::from_secs((HEARTBEAT_INTERVAL * 2).into())) => {
                            s.close().await;
                            break;
                        }
                        v = rx.recv() => {
                            if let None = v {
                                break;
                            }
                        }
                }
            }
        });
        Client {
            uid: OnceLock::new(),
            socket,
            state: Arc::new(AtomicU8::new(0)),
            heartbeat_recved: tx,
            dead: CancellationToken::new(),
            server_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use super::*;

    #[derive(Clone)]
    struct MockSocketHandle {
        id: u32,
        close_called: Arc<AtomicBool>,
        send_bytes: Arc<Mutex<Option<Bytes>>>,
    }

    impl SocketHandle for MockSocketHandle {
        fn id(&self) -> u32 {
            self.id
        }

        async fn send(&self, _message: Bytes) {
            let mut send_bytes = self.send_bytes.lock().unwrap();
            *send_bytes = Some(_message);
        }

        async fn close(&self) {
            self.close_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn test_receive_handshake() {
        let socket_id = 53;
        let client = Client::new(MockSocketHandle {
            id: socket_id,
            close_called: Arc::new(AtomicBool::new(false)),
            send_bytes: Arc::new(Mutex::new(None)),
        });
        let mut mgr = ClientManager::new();
        mgr.add_client(socket_id, client);
        let mut msg = BytesMut::new();
        let uid = b"myuuid";
        msg.put_u8(uid.len() as u8);
        msg.put_slice(uid);
        msg.put_u32(1); // client version
        let packet = packet::encode(packet::PacketType::Handshake, msg.freeze());
        let client = mgr.get_client(socket_id).unwrap();
        client.receive_msg(packet, mgr.clone()).await;
        assert_eq!(
            client.state.load(std::sync::atomic::Ordering::SeqCst),
            WAIT_FOR_HANDSHAKE_ACK
        );

        assert!(mgr.get_client_by_uid("myuuid").unwrap().socket.id() == socket_id);
    }

    #[tokio::test]
    async fn test_receive_handshake_ack() {
        let socket_id = 1238475;
        let client = Client::new(MockSocketHandle {
            id: socket_id,
            close_called: Arc::new(AtomicBool::new(false)),
            send_bytes: Arc::new(Mutex::new(None)),
        });
        let mut mgr = ClientManager::new();
        mgr.add_client(socket_id, client);
        let msg = packet::encode(packet::PacketType::HandshakeAck, Bytes::new()); // Example handshake ack message
        let client = mgr.get_client(socket_id).unwrap();
        client
            .state
            .store(WAIT_FOR_HANDSHAKE_ACK, std::sync::atomic::Ordering::SeqCst);
        client.receive_msg(msg, mgr.clone()).await;
        assert_eq!(
            client.state.load(std::sync::atomic::Ordering::SeqCst),
            READY
        );
    }

    #[tokio::test]
    async fn test_report_error() {
        let socket_id = 1238475;
        let client = Client::new(MockSocketHandle {
            id: socket_id,
            close_called: Arc::new(AtomicBool::new(false)),
            send_bytes: Arc::new(Mutex::new(None)),
        });
        let mut mgr = ClientManager::new();
        mgr.add_client(socket_id, client);
        let client = mgr.get_client(socket_id).unwrap();
        client
            .report_error(ErrorCode::OutedClient as u16, "outed client")
            .await;
        let mut send_bytes = client.socket.send_bytes.lock().unwrap();
        assert!(send_bytes.is_some());
        let send_bytes = send_bytes.take().unwrap();
        let (packet_type, mut body) = packet::decode(send_bytes.clone());
        assert_eq!(packet_type, packet::PacketType::Error);
        assert_eq!(body.get_u16(), ErrorCode::OutedClient as u16);
        assert_eq!(body, Bytes::from_static(b"outed client"));
    }

    // #[tokio::test]
    // async fn test_receive_heartbeat() {
    //     let socket_id = 1238475;
    //     let client = Client::new(MockSocketHandle {
    //         id: socket_id,
    //         close_called: Arc::new(AtomicBool::new(false)),
    //     });
    //     let mut mgr = ClientManager::new();
    //     mgr.add_client(socket_id, client);
    //     let msg = packet::encode(packet::PacketType::Heartbeat, Bytes::new()); // Example heartbeat message
    //     let client = mgr.get_client(socket_id).unwrap();
    //     client
    //         .state
    //         .store(READY, std::sync::atomic::Ordering::SeqCst);
    //     client.receive_msg(msg, mgr.clone()).await;
    //     sleep(Duration::from_secs(3)).await;
    //     assert!(client
    //         .socket
    //         .close_called
    //         .load(std::sync::atomic::Ordering::SeqCst));
    // }

    // #[tokio::test]
    // async fn test_receive_data() {
    //     let client = Arc::new(Client::new(MockSocketHandle::new()));
    //     let mgr = ClientManager::new();
    //     let msg = Bytes::from_static(&[0x00, 0x01, 0x02, 0x03]); // Example data message
    //     client.state.store(READY, Ordering::SeqCst);
    //     client.receive_msg(msg, mgr.clone()).await;
    //     // Assert expectations for the data message
    // }

    // #[tokio::test]
    // async fn test_receive_kick() {
    //     let client = Arc::new(Client::new(MockSocketHandle::new()));
    //     let mgr = ClientManager::new();
    //     let msg = Bytes::from_static(&[0x00, 0x01, 0x02, 0x03]); // Example kick message
    //     client.receive_msg(msg, mgr.clone()).await;
    //     // Assert expectations for the kick message
    // }

    // #[tokio::test]
    // async fn test_receive_error() {
    //     let client = Arc::new(Client::new(MockSocketHandle::new()));
    //     let mgr = ClientManager::new();
    //     let msg = Bytes::from_static(&[0x00, 0x01, 0x02, 0x03]); // Example error message
    //     client.receive_msg(msg, mgr.clone()).await;
    //     // Assert expectations for the error message
    // }
}
