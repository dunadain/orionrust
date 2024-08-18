use std::env;

use gate::{global, s2clistener, transport, ClientManager};
use orion::{appinfo, async_redis};

#[orion::init_tracing]
#[tokio::main]
async fn main() {
    // let s = fs::read_to_string("gate/config/proto.txt").unwrap();
    // s.split("\n").for_each(|line| {
    //     println!("{}", line);
    // });
    // let mut redis = async_redis::connect("redis://localhost:6379").await;
    // let _: () = redis.set("test", "test_data").await.unwrap();
    // let rv: String = redis.get("test").await.unwrap();
    // println!("test: {}", rv);
    // let r: i32 = redis.del("test").await.unwrap();
    // println!("del: {}", r);
    let nats_url = env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
    let nats = orion::nats_client::connect(nats_url).await;
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let redis = async_redis::connect(redis_url).await;
    global::set_nats(nats);
    global::set_redis(redis);

    let addr = env::var("ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u32 = env::var("PORT")
        .unwrap_or_else(|_| "9001".to_string())
        .parse()
        .unwrap();
    let client_mgr = ClientManager::new();
    transport::tcp_transport::start(addr, port, client_mgr.clone());
    s2clistener::listen_for_s2c(client_mgr);
    appinfo().start().await;
}
