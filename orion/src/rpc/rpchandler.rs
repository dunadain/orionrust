use std::{error::Error, future::Future, io::Cursor};

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use prost::Message;

#[async_trait]
pub trait RpcHandler: Send + Sync {
    async fn call(&self, request: Bytes) -> Result<Bytes, Box<dyn Error>>;
}

pub struct RpcHandlerWrapper<F, G, H, I>
where
    F: (Fn(G) -> H) + Send + Sync + 'static,
    G: Message + Default,
    H: Future<Output = I> + Send + Sync + 'static,
    I: Message + Default + Send + Sync,
{
    handler: F,
    _phantom: std::marker::PhantomData<(G, H, I)>,
}

impl<F, G, H, I> RpcHandlerWrapper<F, G, H, I>
where
    F: (Fn(G) -> H) + Send + Sync + 'static,
    G: Message + Default,
    H: Future<Output = I> + Send + Sync + 'static,
    I: Message + Default + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        RpcHandlerWrapper {
            handler,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait]
impl<F, G, H, I> RpcHandler for RpcHandlerWrapper<F, G, H, I>
where
    F: (Fn(G) -> H) + Send + Sync + 'static,
    G: Message + Default,
    H: Future<Output = I> + Send + Sync + 'static,
    I: Message + Default + Send + Sync,
{
    async fn call(&self, request: Bytes) -> Result<Bytes, Box<dyn Error>> {
        let decoded = G::decode(&mut Cursor::new(request))?;
        let response = (self.handler)(decoded).await;
        let mut buf = BytesMut::new();
        buf.reserve(response.encoded_len());
        // Unwrap is safe, since we have reserved sufficient capacity in the vector.
        response.encode(&mut buf).unwrap();
        Ok(buf.freeze())
    }
}
