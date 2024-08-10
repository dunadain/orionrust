use std::time::Duration;

use async_nats::{Message, RequestErrorKind};
use bytes::Bytes;
use futures::StreamExt;
use tracing::error;

#[derive(Clone, Debug)]
pub struct NatsClient {
    client: async_nats::Client, // the client itself is an actor handle
}

impl NatsClient {
    pub async fn publish(&self, subject: String, payload: Bytes) {
        let result = self.client.publish(subject, payload).await;
        if let Err(e) = result {
            error!("Failed to publish message: {}", e);
        }
    }

    pub async fn try_request(
        &self,
        subject: String,
        payload: Bytes,
    ) -> Result<Message, &'static str> {
        for _ in 0..3 {
            let req = async_nats::Request::new()
                .payload(payload.clone())
                .timeout(Some(Duration::from_secs(1)));
            let result = self.client.send_request(subject.clone(), req).await;
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
    }

    pub async fn subscribe<F>(&self, subject: String, callback: F)
    where
        F: Fn(Message) + Send + Sync + 'static,
    {
        let mut subscription = self
            .client
            .subscribe(subject)
            .await
            .expect("Failed to subscribe");
        tokio::spawn({
            let client = self.client.clone();
            async move {
                while let Some(msg) = subscription.next().await {
                    callback(msg);
                }
            }
        });
    }
}

pub async fn connect(url: String) -> NatsClient {
    let result = async_nats::connect(url).await;
    match result {
        Ok(client) => NatsClient { client },
        Err(e) => {
            panic!("Failed to connect to NATS server: {}", e);
        }
    }
}
