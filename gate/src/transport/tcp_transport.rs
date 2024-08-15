use bytes::Bytes;
use orion::{SocketHandle, SocketListener, TcpSocketHandle};

use crate::client::{socket_client::Client, ClientManager, NetClient};
use tracing::error;

pub fn start(addr: String, port: u32, client_mgr: ClientManager<Client<TcpSocketHandle>>) {
    tokio::spawn(async move {
        orion::serve_tcp(addr, port, TcpEventListener { client_mgr }).await;
    });
}

#[derive(Clone)]
struct TcpEventListener {
    client_mgr: ClientManager<Client<TcpSocketHandle>>,
}

impl SocketListener for TcpEventListener {
    fn onopen(&mut self, socket_handle: orion::TcpSocketHandle) {
        let id = socket_handle.id();
        let client = Client::new(socket_handle);
        self.client_mgr.add_client(id, client);
    }

    async fn onmessage(&self, socket_handle: orion::TcpSocketHandle, pkg: Bytes) {
        let client = self.client_mgr.get_client(socket_handle.id());
        match client {
            Some(inner) => {
                inner.receive_msg(pkg, self.client_mgr.clone()).await;
            }
            None => {
                error!("Failed to find client for socket {}", socket_handle.id());
            }
        }
    }

    async fn onclose(&mut self, socket_handle: orion::TcpSocketHandle) {
        let id = socket_handle.id();
        let result = self.client_mgr.get_client(id);
        if let Some(client) = result {
            client.onclose().await;
            self.client_mgr.remove_client(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::{Buf, BufMut, BytesMut};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    use crate::protocol::packet;

    use super::*;

    #[tokio::test]
    async fn test_tcp_transport_start() {
        let addr = "127.0.0.1".to_string();
        let port = 8080;
        let client_mgr = ClientManager::new();
        super::start(addr.clone(), port, client_mgr.clone());

        // Wait for the server to start
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

        // Connect to the server
        let stream = TcpStream::connect(addr.clone() + ":" + &port.to_string()).await;
        assert!(stream.is_ok());
        let (mut reader, mut writer) = stream.unwrap().into_split();
        let handle = tokio::spawn(async move {
            let mut buf = BytesMut::with_capacity(1024);
            let _ = reader.read_buf(&mut buf).await;
            let (pkt_type, mut data) = packet::decode(buf.freeze());
            assert_eq!(pkt_type as u8, packet::PacketType::Handshake as u8);
            assert_eq!(data.get_u8(), 20);
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

        // Assert that the client was added to the client manager
        let client = client_mgr.get_client(0);
        assert!(client.is_some());
        let mut msg = BytesMut::new();
        let uid = b"sl2@34jl2k3";
        msg.put_u8(uid.len() as u8);
        msg.put_slice(uid);
        let packet = packet::encode(packet::PacketType::Handshake, msg.freeze());
        writer.write_all(&packet).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        let c = client_mgr.get_client_by_uid("sl2@34jl2k3");
        assert!(c.is_some());

        let _ = handle.await;

        // close test
        c.unwrap().close().await;
        assert!(client_mgr.get_client(0).is_none());
        assert!(client_mgr.get_client_by_uid("sl2@34jl2k3").is_none());
    }

    // #[tokio::test]
    // async fn test_tcp_transport_onmessage() {
    //     let addr = "127.0.0.1".to_string();
    //     let port = 8080;
    //     let client_mgr = ClientManager::new();
    //     let handle = tokio::spawn(async move {
    //         start(addr.clone(), port, client_mgr).await;
    //     });

    //     // Wait for the server to start
    //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    //     // Connect to the server
    //     let stream = TcpStream::connect((addr.clone(), port)).await;
    //     assert_ok!(stream);

    //     // Wait for the server to handle the connection
    //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    //     // Send a message to the server
    //     let mut stream = stream.unwrap();
    //     let message = "Hello, server!";
    //     stream.write_all(message.as_bytes()).await.unwrap();

    //     // Wait for the server to receive the message
    //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    //     // Assert that the client received the message
    //     let client = client_mgr.get_client(0);
    //     assert!(client.is_some());
    //     let client = client.unwrap();
    //     assert_eq!(client.get_received_msg(), Some(message.to_string()));

    //     // Clean up
    //     handle.abort();
    // }

    // #[tokio::test]
    // async fn test_tcp_transport_onclose() {
    //     let addr = "127.0.0.1".to_string();
    //     let port = 8080;
    //     let client_mgr = ClientManager::new();
    //     let handle = tokio::spawn(async move {
    //         start(addr.clone(), port, client_mgr).await;
    //     });

    //     // Wait for the server to start
    //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    //     // Connect to the server
    //     let stream = TcpStream::connect((addr.clone(), port)).await;
    //     assert_ok!(stream);

    //     // Wait for the server to handle the connection
    //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    //     // Close the connection
    //     let mut stream = stream.unwrap();
    //     stream.shutdown().await.unwrap();

    //     // Wait for the server to handle the close event
    //     tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    //     // Assert that the client was removed from the client manager
    //     let client = client_mgr.get_client(0);
    //     assert!(client.is_none());

    //     // Clean up
    //     handle.abort();
    // }
}
