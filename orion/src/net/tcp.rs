pub mod tcp_actors;

use bytes::{BufMut, Bytes, BytesMut};
pub use tcp_actors::TcpSocketHandle;

use tokio::{
    io::{AsyncReadExt, BufReader},
    net::{TcpListener, TcpStream},
    select,
};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub async fn serve_tcp(
    addr: String,
    port: u32,
    event_listener: impl SocketListener + Clone + Send + Sync + 'static,
) {
    let listener = TcpListener::bind(addr.clone() + ":" + &port.to_string())
        .await
        .expect("should bind to address");
    info!("Listening on: {}", addr + ":" + &port.to_string());
    loop {
        let result = listener.accept().await;
        match result {
            Ok((socket, _)) => {
                listen_for_data(socket, event_listener.clone());
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

fn listen_for_data(
    socket: TcpStream,
    mut event_listener: impl SocketListener + Clone + Send + Sync + 'static,
) {
    let (reader, writer) = socket.into_split();
    let token = CancellationToken::new();
    let socket_handle = TcpSocketHandle::new(writer, token.clone());
    event_listener.onopen(socket_handle.clone());
    tokio::spawn(async move {
        let mut buffer = BytesMut::with_capacity(1024);
        let mut pkg_extractor =
            PackageExtractor::new(event_listener.clone(), socket_handle.clone());
        let mut buf_reader = BufReader::new(reader);
        loop {
            select! {
                result = buf_reader.read_buf(&mut buffer) => {
                    match result {
                        Ok(n) if n != 0 => {
                            pkg_extractor.process(&buffer).await;
                            buffer.clear();
                        }
                        other => {
                            if let Err(e) = other {
                                error!("Failed to read from socket; error = {:?}", e);
                            }
                            break;
                        }
                    }
                }
                _ = token.cancelled() => {
                    break;
                }
            }
        }
        event_listener.onclose(socket_handle).await;
    });
}

pub trait SocketListener {
    fn onopen(&mut self, socket_handle: TcpSocketHandle);
    fn onmessage(
        &self,
        socket_handle: TcpSocketHandle,
        msg: Bytes,
    ) -> impl std::future::Future<Output = ()> + Send;
    fn onclose(
        &mut self,
        socket_handle: TcpSocketHandle,
    ) -> impl std::future::Future<Output = ()> + Send;
}

enum ReadState {
    ReadingHeader,
    ReadingBody,
}

const HEADER_SIZE: usize = 4;
struct PackageExtractor<F: SocketListener> {
    pkg_buffer: BytesMut,
    state: ReadState,
    event_listener: F,
    socket_handle: TcpSocketHandle,
}

impl<F: SocketListener> PackageExtractor<F> {
    fn new(event_listener: F, socket_handle: TcpSocketHandle) -> Self {
        Self {
            pkg_buffer: BytesMut::with_capacity(HEADER_SIZE),
            state: ReadState::ReadingHeader,
            event_listener,
            socket_handle,
        }
    }

    async fn process(&mut self, bytes: &BytesMut) {
        let mut pkgs = vec![];
        self.extract(bytes, 0, &mut pkgs);
        for pkg in pkgs {
            self.event_listener
                .onmessage(self.socket_handle.clone(), pkg)
                .await;
        }
    }

    fn extract(&mut self, bytes: &BytesMut, mut bytes_offset: usize, result_pkgs: &mut Vec<Bytes>) {
        let target_size = match self.state {
            ReadState::ReadingHeader => HEADER_SIZE,
            ReadState::ReadingBody => {
                let msg_len = (self.pkg_buffer[1] as u32) << 16
                    | (self.pkg_buffer[2] as u32) << 8
                    | self.pkg_buffer[3] as u32;
                msg_len as usize + HEADER_SIZE
            }
        };
        let len = bytes.len();
        let data_length_available = len - bytes_offset;
        let data_length_needed = target_size - self.pkg_buffer.len();
        let data_length_to_copy = std::cmp::min(data_length_available, data_length_needed);
        self.pkg_buffer
            .put_slice(&bytes[bytes_offset..(bytes_offset + data_length_to_copy)]);

        bytes_offset += data_length_to_copy;
        if self.pkg_buffer.len() == target_size {
            match self.state {
                ReadState::ReadingHeader => {
                    self.state = ReadState::ReadingBody;
                }
                ReadState::ReadingBody => {
                    result_pkgs.push(self.pkg_buffer.clone().freeze());
                    self.pkg_buffer.clear();
                    self.state = ReadState::ReadingHeader;
                }
            }
        }
        if bytes_offset < len {
            self.extract(bytes, bytes_offset, result_pkgs);
        }
    }
}
