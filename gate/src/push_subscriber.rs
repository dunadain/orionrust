use futures::StreamExt;
use orion::nats_msg;

use crate::{
    client::{ClientManager, NetClient},
    global::nats,
    protocol::message,
};

pub fn start<T: NetClient + 'static>(client_mgr: ClientManager<T>) {
    tokio::spawn(async move {
        let mut subject = "push.".to_string();
        subject.push_str(stringify!(app().uuid()));
        let mut subscription = nats().subscribe(subject).await;
        while let Some(msg) = subscription.next().await {
            let (clientid, proto_id, uid, _, bytes) = nats_msg::decode(msg.payload);
            let client = client_mgr.get_client(clientid);
            if let Some(client) = client {
                client
                    .sendmsg(message::MsgType::Push, proto_id, bytes, 0)
                    .await;
            }
        }
    });
}
