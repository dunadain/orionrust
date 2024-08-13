use std::{fs, sync::OnceLock};

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
struct ServerConfig {
    game: Server,
}

#[derive(Serialize, Deserialize)]
struct Server {
    stateless: bool,
}

pub fn server_config() -> &'static Value {
    static CONFIG: OnceLock<Value> = OnceLock::new();
    CONFIG.get_or_init(|| {
        let content = fs::read_to_string("config/servers.json").unwrap();
        serde_json::from_str(&content).unwrap()
    })
}

pub fn protocols() -> &'static Vec<String> {
    static PROTOCOLS: OnceLock<Vec<String>> = OnceLock::new();
    PROTOCOLS.get_or_init(|| {
        let s = fs::read_to_string("config/proto.txt").unwrap();
        s.split("\n").map(|s| s.to_string()).collect()
    })
}
