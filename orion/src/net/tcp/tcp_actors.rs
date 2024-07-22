use std::sync::atomic::{AtomicU32, Ordering};

use bytes::{Buf, Bytes};
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::tcp::OwnedWriteHalf,
    sync::mpsc,
};
use tokio_util::sync::CancellationToken;
use tracing::error;

enum Message {
    Send(Bytes),
    Close,
}

struct TcpWriteActor {
    receiver: mpsc::Receiver<Message>,
    writer: BufWriter<OwnedWriteHalf>,
    cancel_token: CancellationToken,
}

impl TcpWriteActor {
    async fn handle_message(&mut self, msg: Message) {
        match msg {
            Message::Send(bytes) => {
                let r = self.writer.write_all(&bytes).await;
                if let Err(e) = r {
                    error!("Failed to write to socket; error = {:?}", e);
                    self.cancel_token.cancel();
                }
            }
            Message::Close => {
                let _ = self.writer.shutdown().await;
                self.cancel_token.cancel();
            }
        }
    }
}

#[derive(Clone)]
pub struct SocketHandle {
    id: u32,
    sender: mpsc::Sender<Message>,
}

impl SocketHandle {
    pub fn new(writer: OwnedWriteHalf, cancel_token: CancellationToken) -> Self {
        let (sender, receiver) = mpsc::channel(20);
        let buf_writer = BufWriter::new(writer);
        let write_actor = TcpWriteActor {
            receiver,
            writer: buf_writer,
            cancel_token,
        };
        tokio::spawn(run_write_actor(write_actor));
        static ENUMERATOR: AtomicU32 = AtomicU32::new(0);
        let id = ENUMERATOR.fetch_add(1, Ordering::SeqCst);
        if id == u32::MAX {
            ENUMERATOR.store(0, Ordering::SeqCst);
        }
        SocketHandle { sender, id }
    }

    pub async fn send(&self, message: Bytes) {
        let result = self.sender.send(Message::Send(message)).await;
        if let Err(e) = result {
            error!("Failed to send message; error = {:?}", e);
        }
    }

    pub async fn close(&self) {
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
