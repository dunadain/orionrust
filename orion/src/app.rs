use std::{env, sync::OnceLock};

use tokio::{
    select,
    signal::{
        self,
        unix::{signal, SignalKind},
    },
};
use tracing::info;

pub struct Application {
    uuid: u32,
}

impl Application {
    pub fn new() -> Self {
        Application {
            uuid: env::var("server_id")
                .unwrap_or_else(|_| 0.to_string())
                .parse()
                .expect("server_id should be a number"),
        }
    }

    pub fn uuid(&self) -> u32 {
        self.uuid
    }

    pub async fn start(&self) {
        info!("Application has started");
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        select! {
            _ = signal::ctrl_c() => {
                println!("Received SIGINT");
                self.shutdown().await;
            }
            _ = sigterm.recv() => {
                println!("Received SIGTERM");
                self.shutdown().await;
            }
        }
    }

    async fn shutdown(&self) {}
}

// only immutable data can be stored in a static variable
pub fn app() -> &'static Application {
    static APP: OnceLock<Application> = OnceLock::new();
    APP.get_or_init(|| Application::new())
}
