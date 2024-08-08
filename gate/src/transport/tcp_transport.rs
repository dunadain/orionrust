use crate::{client::NetClient, global};

use bytes::Bytes;
use orion::SocketListener;

use crate::client::{socket_client::Client, ClientManager};
use tracing::error;

pub fn start(addr: String, port: u32) {
    tokio::spawn(async move {
        orion::serve_tcp(
            addr,
            port,
            TcpEventListener {
                client_mgr: global::client_manager(),
            },
        )
        .await;
    });
}

#[derive(Clone)]
struct TcpEventListener {
    client_mgr: ClientManager<Client>,
}

impl SocketListener for TcpEventListener {
    fn onopen(&mut self, socket_handle: orion::SocketHandle) {
        let id = socket_handle.id();
        let client = Client::new(socket_handle);
        self.client_mgr.add_client(id, client);
    }

    fn onmessage(&self, socket_handle: orion::SocketHandle, pkg: Bytes) {
        let client = self.client_mgr.get_client(socket_handle.id());
        tokio::spawn(async move {
            match client {
                Some(inner) => {
                    inner.receive_msg(pkg).await;
                }
                None => {
                    error!("Failed to find client for socket {}", socket_handle.id());
                }
            }
        });
    }

    fn onclose(&mut self, socket_handle: orion::SocketHandle) {
        let cmgr = self.client_mgr.clone();
        tokio::spawn(async move {
            let id = socket_handle.id();
            let result = cmgr.get_client(id);
            if let Some(client) = result {
                client.onclose().await;
                cmgr.remove_client(id);
            }
        });
    }
}
