use crate::comp::Comp;

pub struct Server {
    addr: String,
    port: u16,
    uuid: String,
    components: Vec<Box<dyn Comp>>,
}

impl Server {
    pub fn new(addr: String, port: u16, uuid: String) -> Self {
        Server {
            addr,
            port,
            uuid,
            components: Vec::new(),
        }
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn get_addr(&self) -> &str {
        &self.addr
    }

    pub fn get_uuid(&self) -> &str {
        &self.uuid
    }

    pub async fn start(&mut self) {
        for comp in self.components.iter_mut() {
            comp.init().await;
        }
        for comp in self.components.iter_mut() {
            comp.start().await;
        }
    }

    pub fn add_component(&mut self, comp: Box<dyn Comp>) {
        self.components.push(comp);
    }

    pub fn get_component<T: Comp + 'static>(&self) -> Option<&T> {
        for comp in self.components.iter() {
            if let Some(comp) = comp.as_any().downcast_ref::<T>() {
                return Some(comp);
            }
        }
        None
    }
}
