pub mod tcp_actors;

use bytes::{Bytes, BytesMut};
use tcp_actors::SocketHandle;

use tokio::{io::AsyncReadExt, net::TcpListener, select};
use tokio_util::sync::CancellationToken;
use tracing::error;

pub async fn serve_tcp(addr: &str, port: u32, event_listener: EventListener) {
    let listener = TcpListener::bind(addr.to_string() + ":" + &port.to_string())
        .await
        .expect("should bind to address");
    tokio::spawn(async move {
        loop {
            let result = listener.accept().await;
            match result {
                Ok((socket, _)) => {
                    let (mut reader, writer) = socket.into_split();
                    let token = CancellationToken::new();
                    let socket_handle = SocketHandle::new(writer, token.clone());
                    (event_listener.onopen)(socket_handle.clone());
                    tokio::spawn(async move {
                        let mut buffer = BytesMut::with_capacity(1024);
                        let mut pkg_extractor = PackageExtractor::new({
                            |pkg| {
                                (event_listener.onmessage)(socket_handle.clone(), pkg);
                            }
                        });
                        loop {
                            select! {
                                result = reader.read_buf(&mut buffer) => {
                                    match result {
                                        Ok(n) if n != 0 => {
                                            pkg_extractor.extract(&buffer, n, 0);
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
                        (event_listener.onclose)(socket_handle);
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    });
}

pub struct EventListener {
    pub onopen: fn(SocketHandle),
    pub onmessage: fn(SocketHandle, Bytes),
    pub onclose: fn(SocketHandle),
}

enum ReadState {
    ReadingHeader,
    ReadingBody,
}

const HEADER_SIZE: usize = 4;
struct PackageExtractor<F> {
    pkg_buffer: BytesMut,
    pkg_buffer_offset: usize, // for header and msg
    state: ReadState,
    onpkgcomplete: F,
}

impl<F> PackageExtractor<F>
where
    F: Fn(Bytes) -> (),
{
    fn new(onpkgcomplete: F) -> Self {
        Self {
            pkg_buffer: BytesMut::with_capacity(HEADER_SIZE),
            pkg_buffer_offset: 0,
            state: ReadState::ReadingHeader,
            onpkgcomplete,
        }
    }

    fn extract(&mut self, bytes: &BytesMut, len: usize, mut bytes_offset: usize) {
        let target_size = match self.state {
            ReadState::ReadingHeader => HEADER_SIZE,
            ReadState::ReadingBody => self.pkg_buffer.len(),
        };
        let data_length_available = len - bytes_offset;
        let data_length_needed = target_size - self.pkg_buffer_offset;
        let data_length_to_copy = std::cmp::min(data_length_available, data_length_needed);
        self.pkg_buffer[self.pkg_buffer_offset..self.pkg_buffer_offset + data_length_to_copy]
            .copy_from_slice(&bytes[bytes_offset..bytes_offset + data_length_to_copy]);
        self.pkg_buffer_offset += data_length_to_copy;
        bytes_offset += data_length_to_copy;
        if self.pkg_buffer_offset == target_size {
            match self.state {
                ReadState::ReadingHeader => {
                    let msg_length = (self.pkg_buffer[1] as u32) << 16
                        | (self.pkg_buffer[2] as u32) << 8
                        | self.pkg_buffer[3] as u32;
                    self.pkg_buffer.resize(HEADER_SIZE + msg_length as usize, 0);
                    self.state = ReadState::ReadingBody;
                }
                ReadState::ReadingBody => {
                    (self.onpkgcomplete)(self.pkg_buffer.clone().freeze());
                    self.pkg_buffer.clear();
                    self.pkg_buffer_offset = 0;
                    self.state = ReadState::ReadingHeader;
                }
            }
        }
        if bytes_offset < len {
            self.extract(bytes, len, bytes_offset);
        }
    }
}
