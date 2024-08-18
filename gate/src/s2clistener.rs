use futures::StreamExt;
use orion::{appinfo, nats_msg};
use tracing::debug;

use crate::{client::NetClient, global::nats, protocol::message::MsgType, ClientManager};

/// 从其他服务器收到的push和response消息(server 2 client)
pub fn listen_for_s2c<T: NetClient + 'static>(client_mgr: ClientManager<T>) {
    tokio::spawn(async move {
        let mut generic_subject = appinfo().uuid().to_string();
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
