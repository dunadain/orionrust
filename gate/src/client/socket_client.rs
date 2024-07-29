use std::{
    sync::{atomic::AtomicU8, Arc},
    time::Duration,
};

use bytes::{BufMut, Bytes, BytesMut};
use orion::SocketHandle;
use redis::aio::MultiplexedConnection;
use tokio::{select, sync::mpsc, time::sleep};
use tokio_util::sync::CancellationToken;

use crate::protocol::{message, packet};

use super::NetClient;

const WAIT_FOR_HANDSHAKE: u8 = 0;
const WAIT_FOR_HANDSHAKE_ACK: u8 = 1;
const READY: u8 = 2;

const HEARTBEAT_INTERVAL: u8 = 20;

#[derive(Debug, Clone)]
pub struct Client {
    socket: SocketHandle,
    state: Arc<AtomicU8>,
    heartbeat_recved: mpsc::Sender<()>,
    dead: CancellationToken,
    redis: MultiplexedConnection,
}

impl NetClient for Client {
    async fn receive_msg(self: Arc<Self>, msg: Bytes) {
        let (packet_type, decoded_body) = packet::decode(msg);
        match packet_type {
            packet::PacketType::Handshake => {
                if self.state.load(std::sync::atomic::Ordering::SeqCst) != WAIT_FOR_HANDSHAKE {
                    return;
                }
                let mut send_bytes = BytesMut::new();
                send_bytes.put_u8(20); // heartbeat interval
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

    async fn onopen(self: Arc<Self>) {
        todo!()
    }

    async fn onclose(self: Arc<Self>) {
        // TODO: 把此用户相关的数据从缓冲或者其他服务器清理
        self.dead.cancel();
    }

    async fn close(self: Arc<Self>) {
        self.socket.close().await;
        let token = self.dead.clone();
        token.cancelled().await;
    }
}

impl Client {
    pub fn new(socket: SocketHandle, redis: MultiplexedConnection) -> Self {
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
            redis,
        }
    }
}
