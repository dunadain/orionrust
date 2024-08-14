use std::{fs, sync::OnceLock};

use serde_json::Value;

pub fn server_config() -> &'static Value {
    static SERVER_CONFIG: OnceLock<Value> = OnceLock::new();
    SERVER_CONFIG.get_or_init(|| {
        let s = fs::read_to_string("config/server.json").unwrap();
        serde_json::from_str(&s).unwrap()
    })
}

pub fn protocols() -> &'static Vec<String> {
    static PROTOCOLS: OnceLock<Vec<String>> = OnceLock::new();
    PROTOCOLS.get_or_init(|| {
        let s = fs::read_to_string("config/proto.txt").unwrap();
        s.split("\n").map(|s| s.to_string()).collect()
    })
}
