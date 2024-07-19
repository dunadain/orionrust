mod client_mgr;
mod tcp_comp;

use std::any::Any;

pub trait Comp: Send + Sync {
    fn init(&mut self);
    fn start(&self);
    fn dispose(&self);
    fn as_any(&self) -> &dyn Any;
}
