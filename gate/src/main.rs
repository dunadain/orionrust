use std::env;

use gate::{
    client::{socket_client::Client, ClientManager},
    global,
};
use orion::{app, async_redis};

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
    let clientmgr: ClientManager<Client> = ClientManager::new();
    global::set_client_manager(clientmgr);
    app().start().await;
}
