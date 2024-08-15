use std::error::Error;

use bytes::Bytes;

use orion::nats_client;

use crate::global::nats;

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
