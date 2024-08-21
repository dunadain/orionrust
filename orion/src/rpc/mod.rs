use std::{collections::HashMap, sync::OnceLock};

use bytes::{BufMut, Bytes, BytesMut};

use rpchandler::RpcHandler;

mod rpchandler;

enum RpcError {
    NotFound = 1,
    HandlerError = 2,
}

pub struct RpcRouter {
    map: HashMap<String, Box<dyn RpcHandler>>,
}

impl RpcRouter {
    pub fn new() -> Self {
        RpcRouter {
            map: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, handler: Box<dyn RpcHandler>) {
        self.map.insert(name, handler);
    }

    pub async fn handle(&self, name: &str, request: Bytes) -> Bytes {
        let handler = self.map.get(name);
        match handler {
            None => {
                tracing::error!("rpc handler not found for {}", name);
                let mut buf = BytesMut::new();
                buf.put_u8(RpcError::NotFound as u8);
                buf.freeze()
            }
            Some(handler) => {
                let result = handler.call(request).await;
                match result {
                    Ok(response) => response,
                    Err(e) => {
                        tracing::error!("rpc handler error: {}", e);
                        let mut buf = BytesMut::new();
                        buf.put_u8(RpcError::HandlerError as u8);
                        buf.freeze()
                    }
                }
            }
        }
    }
}

pub fn rpc<F, G, H, I>(handle: F) -> Box<dyn RpcHandler>
where
    F: (Fn(G) -> H) + Send + Sync + 'static,
    G: prost::Message + Default + 'static,
    H: std::future::Future<Output = I> + Send + Sync + 'static,
    I: prost::Message + Default + Send + Sync + 'static,
{
    Box::new(rpchandler::RpcHandlerWrapper::new(handle))
}

pub struct Pair(&'static str, Box<dyn RpcHandler>);

pub fn register_rpc_routes(routes: Vec<Pair>) {
    let mut router = RpcRouter::new();
    for pair in routes {
        router.register(pair.0.to_string(), pair.1);
    }
    RPCROUTER.get_or_init(|| router);
}

static RPCROUTER: OnceLock<RpcRouter> = OnceLock::new();
pub fn rpc_router() -> &'static RpcRouter {
    RPCROUTER.get().expect("rpc router not initialized")
}

#[macro_export]
macro_rules! register {
    ($($name:literal => $handler:expr),*) => {
        register_rpc_routes(vec![$(Pair($name, $handler)),*]);
    };
}

#[cfg(test)]
mod tests {
    use prost::Message;
    use protobuf::rpc::{HelloReply, HelloRequest};

    use super::*;

    async fn test(req: HelloRequest) -> HelloReply {
        HelloReply {
            message: format!("Hello, {}!", req.name),
        }
    }

    #[tokio::test]
    async fn test_handle_existing_handler() {
        let mut router = RpcRouter::new();
        router.register(
            "mock".to_string(),
            Box::new(rpchandler::RpcHandlerWrapper::new(test)),
        );

        let req = HelloRequest {
            name: "world".to_string(),
        };
        let mut buf = BytesMut::new();
        buf.reserve(req.encoded_len());
        // Unwrap is safe, since we have reserved sufficient capacity in the vector.
        req.encode(&mut buf).unwrap();
        let request = buf.freeze();
        let response = router.handle("mock", request).await;
        let reply = HelloReply::decode(response).unwrap();
        assert_eq!(reply.message, "Hello, world!");
    }

    #[tokio::test]
    async fn test_handle_nonexistent_handler() {
        let router = RpcRouter::new();

        let request = Bytes::from("test request");
        let response = router.handle("nonexistent", request).await;

        assert_eq!(response[0], RpcError::NotFound as u8);
    }

    #[tokio::test]
    async fn test_handle_handler_error() {
        let mut router = RpcRouter::new();
        router.register(
            "error".to_string(),
            Box::new(rpchandler::RpcHandlerWrapper::new(test)),
        );

        let request = Bytes::from("test request");
        let response = router.handle("error", request).await;

        assert_eq!(response[0], RpcError::HandlerError as u8);
    }
}
