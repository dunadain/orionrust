use bytes::Bytes;
use orion::SocketHandle;

#[derive(Clone)]
pub struct Client {
    socket: SocketHandle,
}

impl Client {
    pub fn new(socket: SocketHandle) -> Self {
        Client { socket }
    }

    pub fn receive_msg(&self, msg: Bytes) {}
}
