use std::{env, sync::OnceLock};

use tokio::{
    select,
    signal::{
        self,
        unix::{signal, SignalKind},
    },
};
use tracing::info;

pub struct AppInfo {
    uuid: u32,
    server_type: String,
}

impl AppInfo {
    pub fn new() -> Self {
        let uuid: u32 = env::var("server_id")
            .unwrap_or_else(|_| 1.to_string())
            .parse()
            .expect("server_id should be a number");
        if uuid == 0 {
            panic!("server_id should not be 0");
        }
        AppInfo {
            uuid,
            server_type: env::var("server_type").unwrap_or_else(|_| "".to_string()),
        }
    }

    /// Returns the UUID of the server.
    ///
    /// uuid is a unique identifier for the server
    ///
    /// and must be greater than 0
    pub fn uuid(&self) -> u32 {
        self.uuid
    }

    pub fn server_type(&self) -> &str {
        &self.server_type
    }
}

// only immutable data can be stored in a static variable
pub fn appinfo() -> &'static AppInfo {
    static APP: OnceLock<AppInfo> = OnceLock::new();
    APP.get_or_init(|| AppInfo::new())
}

pub struct Application;

impl Application {
    pub fn new() -> Self {
        Application
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
