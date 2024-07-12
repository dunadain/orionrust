use std::env;

use async_nats::{Client, Message, RequestErrorKind};
use async_trait::async_trait;
use bytes::Bytes;
use tracing::error;

use super::Comp;

#[derive(Debug)]
pub struct NatsComp {
    nc: Option<Client>,
}

impl NatsComp {
    pub fn new() -> Self {
        NatsComp { nc: None }
    }

    pub fn clone_nat_client(&self) -> Client {
        self.nc
            .as_ref()
            .expect("nats should be initialized")
            .clone()
    }

    pub fn publish(&self, subject: String, payload: Bytes) {
        let nc = self.clone_nat_client();
        tokio::spawn(async move {
            let result = nc.publish(subject, payload).await;
            if let Err(e) = result {
                error!("Failed to publish message: {}", e);
            }
        });
    }

    pub fn try_request(
        &self,
        subject: String,
        payload: Bytes,
    ) -> tokio::task::JoinHandle<Result<Message, &'static str>> {
        let nc = self.clone_nat_client();
        return tokio::spawn(async move {
            for _ in 0..3 {
                let result = nc.request(subject.clone(), payload.clone()).await;
                if let Err(e) = result {
                    error!("Failed to request message: {}", e);
                    if let RequestErrorKind::NoResponders = e.kind() {
                        return Err("No responders");
                    }
                } else {
                    return Ok(result.unwrap());
                }
            }
            Err("Failed to request message")
        });
    }
}

#[async_trait]
impl Comp for NatsComp {
    async fn init(&mut self) {
        let nats_url = env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".to_string());
        let result = async_nats::connect(nats_url).await;
        match result {
            Ok(client) => {
                self.nc = Some(client);
            }
            Err(e) => {
                panic!("Failed to connect to NATS server: {}", e);
            }
        }
    }

    async fn start(&self) {
        println!("NatsComp start");
    }

    async fn dispose(&self) {
        println!("NatsComp dispose");
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
