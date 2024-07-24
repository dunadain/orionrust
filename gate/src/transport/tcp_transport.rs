use std::env;

use bytes::Bytes;
use orion::SocketListener;

use crate::client::{socket_client::Client, ClientManager};
use tracing::error;

struct TcpTransport {
    client_mgr: ClientManager,
}

impl TcpTransport {
    fn start(&self) {
        let addr = env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
        let port: u32 = env::var("PORT")
            .unwrap_or_else(|_| "8001".to_string())
            .parse()
            .unwrap();
        let cmgr = self.client_mgr.clone();
        tokio::spawn(async move {
            orion::serve_tcp(addr, port, TcpEventListener { client_mgr: cmgr }).await;
        });
    }
}

#[derive(Clone)]
struct TcpEventListener {
    client_mgr: ClientManager,
}

impl SocketListener for TcpEventListener {
    fn onopen(&mut self, socket_handle: orion::SocketHandle) {
        let id = socket_handle.id();
        let client = Client::new(socket_handle);
        self.client_mgr.add_client(id, client);
    }

    fn onmessage(&mut self, socket_handle: orion::SocketHandle, pkg: Bytes) {
        let client = self.client_mgr.get_client(socket_handle.id());
        match client {
            Some(inner) => {
                tokio::spawn(async move {
                    inner.receive_msg(pkg);
                });
            }
            None => {
                error!("Failed to find client for socket {}", socket_handle.id());
            }
        }
    }

    fn onclose(&mut self, socket_handle: orion::SocketHandle) {
        self.client_mgr.remove_client(socket_handle.id());
    }
}
