use futures::StreamExt;
use tracing::debug;

use super::nats_client::NatsClient;

pub fn subscribe(nats: NatsClient, subject: String) {
    tokio::spawn(async move {
        let mut sub = nats.subscribe(subject).await;
        while let Some(msg) = sub.next().await {
            if let Some(reply) = msg.reply {
                let _ = nats.publish(reply, "ok".into()).await;
            }
        }
        debug!("req notify subscriber exit");
    });
}
