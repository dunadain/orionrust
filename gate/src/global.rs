use std::sync::OnceLock;

use orion::nats_client::NatsClient;
use redis::aio::ConnectionManager;

static REDIS: OnceLock<ConnectionManager> = OnceLock::new();
static NATS: OnceLock<NatsClient> = OnceLock::new();

pub fn set_redis(client: ConnectionManager) {
    REDIS.get_or_init(|| client);
}

pub fn redis_copy() -> ConnectionManager {
    REDIS.get().expect("Redis not registered").clone()
}

pub fn set_nats(client: NatsClient) {
    NATS.get_or_init(|| client);
}

pub fn nats() -> &'static NatsClient {
    NATS.get().expect("Nats not registered")
}
