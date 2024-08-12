use bytes::Bytes;

pub trait SocketHandle {
    fn send(&self, msg: Bytes) -> impl std::future::Future<Output = ()> + Send;
    fn close(&self) -> impl std::future::Future<Output = ()> + Send;
    fn id(&self) -> u32;
}
