use std::env;

use bytes::Bytes;
use orion::SocketListener;

use crate::client::ClientManager;
use tracing::error;

struct TcpComp {
    client_mgr: ClientManager,
}

impl TcpComp {
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
        self.client_mgr.add_client(socket_handle);
    }

    fn onmessage(&mut self, socket_handle: orion::SocketHandle, pkg: Bytes) {
        let client = self.client_mgr.get_client(socket_handle.id());
        match client {
            Some(inner) => {
                tokio::spawn(async move {
                    inner.receive_msg(pkg).await;
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