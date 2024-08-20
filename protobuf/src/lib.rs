use std::io::Cursor;

use bytes::{Bytes, BytesMut};
use prost::Message;

pub mod rpc {
    include!(concat!(env!("OUT_DIR"), "/rpc.items.rs"));
}

pub struct ProtoWrapper<F>(F);

impl<F: Message + Default> Into<ProtoWrapper<F>> for Bytes {
    fn into(self) -> ProtoWrapper<F> {
        ProtoWrapper(F::decode(&mut Cursor::new(self)).unwrap())
    }
}

// implement from for Wrapper
impl<F: Message + Default> From<ProtoWrapper<F>> for Bytes {
    fn from(wrapper: ProtoWrapper<F>) -> Bytes {
        let mut buf = BytesMut::new();
        let ProtoWrapper(inner) = wrapper;
        buf.reserve(inner.encoded_len());
        // Unwrap is safe, since we have reserved sufficient capacity in the vector.
        inner.encode(&mut buf).unwrap();
        buf.freeze()
    }
}
