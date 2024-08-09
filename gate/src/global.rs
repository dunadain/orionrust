use std::sync::OnceLock;

use orion::nats_client::NatsClient;
use redis::aio::ConnectionManager;

use crate::client::{socket_client::Client, ClientManager};

static REDIS: OnceLock<ConnectionManager> = OnceLock::new();
static NATS: OnceLock<NatsClient> = OnceLock::new();
static CLIENTMANAGER: OnceLock<ClientManager<Client>> = OnceLock::new();

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

pub fn set_client_manager(mgr: ClientManager<Client>) {
    CLIENTMANAGER.get_or_init(|| mgr);
}

pub fn client_manager_copy() -> ClientManager<Client> {
    CLIENTMANAGER
        .get()
        .expect("ClientManager not registered")
        .clone()
}
