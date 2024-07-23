use std::{
    sync::{atomic::AtomicU8, Arc},
    time::Duration,
};

use bytes::{BufMut, Bytes, BytesMut};
use orion::SocketHandle;
use tokio::{select, sync::mpsc, time::sleep};

use crate::protocol::{message, packet};

const WAIT_FOR_HANDSHAKE: u8 = 0;
const WAIT_FOR_HANDSHAKE_ACK: u8 = 1;
const READY: u8 = 2;

const HEARTBEAT_INTERVAL: u8 = 20;

#[derive(Clone)]
pub struct Client {
    socket: SocketHandle,
    state: Arc<AtomicU8>,
    heartbeat_recved: mpsc::Sender<()>,
}

impl Client {
    pub fn new(socket: SocketHandle) -> Self {
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
        }
    }

    pub async fn receive_msg(&self, msg: Bytes) {
        let (packet_type, decoded) = packet::decode(msg);
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
                let packet = packet::encode(packet::PacketType::HandshakeAck, Bytes::new());
                self.socket.send(packet).await;
                self.state.store(READY, std::sync::atomic::Ordering::SeqCst);
            }
            packet::PacketType::Heartbeat => {
                let _ = self.heartbeat_recved.send(()).await;
                let packet = packet::encode(packet::PacketType::Heartbeat, Bytes::new());
                self.socket.send(packet).await;
            }
            packet::PacketType::Data => {
                let (msg_type, proto_id, id, data) = message::decode(decoded);
            }
            packet::PacketType::Kick => todo!(),
            packet::PacketType::Error => todo!(),
        }
    }
}
