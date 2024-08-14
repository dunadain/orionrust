use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
};

use async_nats::{client, Message};
use bytes::Bytes;
use futures::StreamExt;
use orion::{app, nats_client, nats_msg};
use tracing::debug;

use crate::{
    client::{ClientManager, NetClient},
    global::nats,
    protocol::message::MsgType,
};

pub trait NatRequest {
    fn try_request(
        &self,
        subject: String,
        reply: String,
        payload: Bytes,
    ) -> impl std::future::Future<Output = Result<(), Box<dyn Error>>> + Send;
}

impl NatRequest for nats_client::NatsClient {
    /// request
    /// reply subject example: {serverid}.reply.{requestid}
    ///
    /// push subject example: {serverid}.push
    ///
    /// request subject example(stateless): handler.{servertype}
    async fn try_request(
        &self,
        subject: String,
        reply: String,
        payload: Bytes,
    ) -> Result<(), Box<dyn Error>> {
        let result = nats().publish_with_reply(subject, reply, payload).await;
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

/// 从其他服务器收到的push和response消息(server 2 client)
pub fn listen_for_s2c<T: NetClient + 'static>(client_mgr: ClientManager<T>) {
    tokio::spawn(async move {
        let mut generic_subject = app().uuid().to_string();
        generic_subject.push_str(".>");
        let mut subscription = nats().subscribe(generic_subject).await;
        while let Some(msg) = subscription.next().await {
            let sub = msg.subject;
            let payload = msg.payload;
            let (clientid, proto_id, reqid, _, _, data) = nats_msg::decode(payload);
            let client = client_mgr.get_client(clientid);
            if let Some(client) = client {
                let v: Vec<&str> = sub.split('.').collect();
                if v.len() >= 2 {
                    match v[1] {
                        "push" => {
                            client.sendmsg(MsgType::Push, proto_id, data, 0).await;
                        }
                        "reply" => {
                            client
                                .sendmsg(MsgType::Response, proto_id, data, reqid)
                                .await
                        }
                        _ => {}
                    }
                }
            }
        }
        debug!("nats push/response subscriber exit");
    });
}
