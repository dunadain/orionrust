use redis::{aio::ConnectionManager, Client};

/// connection manager will reconnect to redis if the connection is lost
pub async fn connect(url: &str) -> ConnectionManager {
    let client = Client::open(url).expect("Failed to create redis client");
    // let config = ConnectionManagerConfig::default();
    // use default config
    let con_result = client.get_connection_manager().await;
    match con_result {
        Ok(con) => con,
        Err(e) => panic!("Failed to connect to redis: {}", e),
    }
}
