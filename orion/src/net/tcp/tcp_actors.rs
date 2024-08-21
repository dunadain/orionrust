use std::sync::atomic::{AtomicU32, Ordering};

use bytes::Bytes;
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::tcp::OwnedWriteHalf,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;
use tracing::error;

use crate::net::socket_handle::SocketHandle;

enum Message {
    Send(Bytes),
    Close,
}

struct TcpWriteActor {
    receiver: mpsc::Receiver<Message>,
    writer: OwnedWriteHalf,
    cancel_token: CancellationToken,
    is_closing: bool,
}

impl TcpWriteActor {
    async fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::Send(bytes) => {
                let r = self.writer.write_all(&bytes).await;
                if let Err(e) = r {
                    error!("Failed to write to socket; error = {:?}", e);
                    self.cancel_token.cancel();
                    return;
                }
            }
            Message::Close => {
                if self.is_closing {
                    return;
                }
                self.is_closing = true;
                let _ = self.writer.shutdown().await;
                self.cancel_token.cancel();
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TcpSocketHandle {
    id: u32,
    sender: mpsc::Sender<Message>,
}

impl TcpSocketHandle {
    pub fn new(writer: OwnedWriteHalf, cancel_token: CancellationToken) -> Self {
        let (sender, receiver) = mpsc::channel(50); // 必须压力测试测测看
        let write_actor = TcpWriteActor {
            receiver,
            writer,
            cancel_token,
            is_closing: false,
        };
        tokio::spawn(run_write_actor(write_actor));
        static ENUMERATOR: AtomicU32 = AtomicU32::new(1);
        let id = ENUMERATOR.fetch_add(1, Ordering::SeqCst);
        if id == u32::MAX {
            ENUMERATOR.store(1, Ordering::SeqCst);
        }
        TcpSocketHandle { sender, id }
    }
}

impl SocketHandle for TcpSocketHandle {
    fn id(&self) -> u32 {
        self.id
    }
    async fn send(&self, message: Bytes) {
        let result = self.sender.send(Message::Send(message)).await;
        if let Err(e) = result {
            error!("Failed to send message; error = {:?}", e);
        }
    }

    async fn close(&self) {
        let result = self.sender.send(Message::Close).await;
        if let Err(e) = result {
            error!("Failed to send close message; error = {:?}", e);
        }
    }
}

async fn run_write_actor(mut actor: TcpWriteActor) {
    while let Some(msg) = actor.receiver.recv().await {
        actor.handle_message(msg).await;
    }
}
