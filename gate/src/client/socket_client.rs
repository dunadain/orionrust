use std::{
    collections::HashMap,
    sync::{atomic::AtomicU8, Arc, Mutex},
    time::Duration,
};

use bytes::{BufMut, Bytes, BytesMut};
use orion::SocketHandle;
use tokio::{select, sync::mpsc, time::sleep};
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::protocol::{message, packet};

use super::{ClientManager, NetClient};

const WAIT_FOR_HANDSHAKE: u8 = 0;
const WAIT_FOR_HANDSHAKE_ACK: u8 = 1;
const READY: u8 = 2;

const HEARTBEAT_INTERVAL: u8 = 20;

#[derive(Clone)]
pub struct Client<T: SocketHandle + Sync + Send + Clone + 'static> {
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
    async fn receive_msg(self: &Arc<Self>, msg: Bytes, mgr: ClientManager<Client<T>>) {
        let (packet_type, decoded_body) = packet::decode(msg);
        match packet_type {
            packet::PacketType::Handshake => {
                if self.state.load(std::sync::atomic::Ordering::SeqCst) != WAIT_FOR_HANDSHAKE {
                    return;
                }
                let uid_len = decoded_body[0];
                let uid_bytes = decoded_body.slice(1..(uid_len + 1) as usize);
                let uid = String::from_utf8(uid_bytes.to_vec());
                match uid {
                    Ok(uid) => {
                        // TODO: 剔除重复登录用户
                        mgr.bind_connection(uid, self.socket.id());
                    }
                    Err(e) => {
                        error!("Failed to parse uid: {}", e);
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
                let (msg_type, proto_id, id, data) = message::decode(decoded_body);
            }
            packet::PacketType::Kick => todo!(),
            packet::PacketType::Error => todo!(),
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

    use crate::client;

    use super::*;

    #[derive(Clone)]
    struct MockSocketHandle {
        id: u32,
        close_called: Arc<AtomicBool>,
    }

    impl SocketHandle for MockSocketHandle {
        fn id(&self) -> u32 {
            self.id
        }

        async fn send(&self, _message: Bytes) {}

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
        });
        let mut mgr = ClientManager::new();
        mgr.add_client(socket_id, client);
        let mut msg = BytesMut::new();
        let uid = b"myuuid";
        msg.put_u8(uid.len() as u8);
        msg.put_slice(uid);
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
