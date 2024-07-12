use std::any::Any;

use async_trait::async_trait;

mod nats_comp;
pub use nats_comp::*;
mod tcp_server;
pub use tcp_server::*;

#[async_trait]
pub trait Comp {
    async fn init(&mut self);
    async fn start(&self);
    async fn dispose(&self);
    fn as_any(&self) -> &dyn Any;
}
