use std::time::Duration;

use async_nats::{Message, RequestErrorKind, Subject};
use bytes::Bytes;
use tokio::time::sleep;
use tracing::error;

#[derive(Clone, Debug)]
pub struct NatsClient {
    client: async_nats::Client, // the client itself is an actor handle
}

impl NatsClient {
    pub async fn publish(&self, subject: Subject, payload: Bytes) -> Result<(), &'static str> {
        for i in 1..4 {
            let result = self.client.publish(subject.clone(), payload.clone()).await;
            match result {
                Err(e) => {
                    error!("Failed to publish message: {}", e);
                    sleep(Duration::from_millis(100 * i)).await;
                }
                _ => {
                    return Ok(());
                }
            }
        }
        Err("Failed to publish message")
    }

    pub async fn publish_with_reply(
        &self,
        subject: String,
        reply: String,
        payload: Bytes,
    ) -> Result<(), &'static str> {
        for i in 1..4 {
            let result = self
                .client
                .publish_with_reply(subject.clone(), reply.clone(), payload.clone())
                .await;
            match result {
                Err(e) => {
                    error!("Failed to publish message: {}", e);
                    sleep(Duration::from_millis(100 * i)).await;
                }
                Ok(()) => {
                    return Ok(());
                }
            }
        }
        Err("Failed to request message")
    }

    // pub async fn try_request(
    //     &self,
    //     subject: String,
    //     payload: Bytes,
    // ) -> Result<Message, &'static str> {
    //     for i in 1..4 {
    //         let req = async_nats::Request::new()
    //             .payload(payload.clone())
    //             .timeout(Some(Duration::from_secs(1)));
    //         let result = self.client.send_request(subject.clone(), req).await;
    //         match result {
    //             Err(e) => {
    //                 error!("Failed to request message: {}", e);
    //                 if let RequestErrorKind::NoResponders = e.kind() {
    //                     return Err("No responders");
    //                 }
    //                 sleep(Duration::from_millis(100 * i)).await;
    //             }
    //             Ok(msg) => {
    //                 return Ok(msg);
    //             }
    //         }
    //     }
    //     Err("Failed to request message")
    // }

    pub async fn subscribe(&self, subject: String) -> async_nats::Subscriber {
        let result = self.client.subscribe(subject).await;
        match result {
            Ok(sub) => sub,
            Err(e) => {
                panic!("Failed to subscribe to NATS server: {}", e);
            }
        }
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
