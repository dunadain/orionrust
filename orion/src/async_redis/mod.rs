use redis::{aio::MultiplexedConnection, Client};

pub async fn connect(url: &str) -> MultiplexedConnection {
    let client = Client::open(url).expect("Failed to create redis client");
    let con_result = client.get_multiplexed_async_connection().await;
    match con_result {
        Ok(con) => con,
        Err(e) => panic!("Failed to connect to redis: {}", e),
    }
}
