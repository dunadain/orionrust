use std::time::Duration;

use redis::{
    aio::{ConnectionManager, ConnectionManagerConfig},
    Client,
};
use tokio::{select, time::sleep};

/// connection manager will reconnect to redis if the connection is lost
pub async fn connect(url: &str) -> ConnectionManager {
    let client = Client::open(url).expect("Failed to create redis client");
    let config = ConnectionManagerConfig::new()
        .set_connection_timeout(Duration::from_secs(5))
        .set_response_timeout(Duration::from_secs(5))
        .set_max_delay(1200000)
        .set_number_of_retries(1000);

    select! {
        con_result = client.get_connection_manager_with_config(config) => {
            match con_result {
                Ok(con) => {
                    return con;
                },
                Err(e) => panic!("Failed to connect to redis: {}", e),
            }
        }
        _ = sleep(Duration::from_secs(20)) => {
            panic!("Failed to connect to redis: timeout");
        }
    }
}
