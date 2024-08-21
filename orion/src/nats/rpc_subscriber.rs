use async_nats::Message;
use futures::StreamExt;
use tracing::debug;

use crate::{appinfo, rpc::rpc_router};

use super::nats_client::NatsClient;

pub fn subscribe(nats: NatsClient) {
    tokio::spawn(async move {
        let subject = "rpc.".to_string() + &appinfo().uuid().to_string() + ".>";
        let mut sub = nats.subscribe(subject).await;
        while let Some(msg) = sub.next().await {
            process_rpc_request(msg, nats.clone());
        }
        debug!("rpc subscriber exits");
    });
}

pub fn queue_subscribe(nats: NatsClient) {
    tokio::spawn(async move {
        let subject = "rpc.".to_string() + appinfo().server_type() + ".>";
        let mut sub = nats
            .queue_subscribe(subject, appinfo().server_type().to_string())
            .await;
        while let Some(msg) = sub.next().await {
            process_rpc_request(msg, nats.clone());
        }
        debug!("queue group rpc subscriber exits");
    });
}

fn process_rpc_request(msg: Message, nats: NatsClient) {
    tokio::spawn(async move {
        let subject = msg.subject.as_str();
        let route = find_route(subject);
        if let Some(route) = route {
            let response = rpc_router().handle(route, msg.payload).await;
            if let Some(reply) = msg.reply {
                let _ = nats.publish(reply, response).await;
            }
        }
    });
}

fn find_route(subject: &str) -> Option<&str> {
    let mut sec_dot = 0;
    let mut fourth_dot = 0;
    let mut dot_count = 0;
    for (i, c) in subject.char_indices() {
        if c == '.' {
            if dot_count < 4 {
                dot_count += 1;
                if dot_count == 2 {
                    sec_dot = i;
                } else if dot_count == 4 {
                    fourth_dot = i;
                }
            } else {
                break;
            }
        }
    }
    if dot_count < 4 {
        return None;
    }
    Some(&subject[sec_dot + 1..fourth_dot])
}
