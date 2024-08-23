use bytes::{Buf, BufMut, Bytes, BytesMut};

pub fn encode(id: u32, proto_id: u16, reqid: u8, uid: &str, svrid: u32, data: Bytes) -> Bytes {
    let mut buf = BytesMut::with_capacity(12 + uid.len() + data.len()); // id(4) + proto_id(2) + reqid(1) + uidlen(1) + uid.len() + svrid(4) + data.len()
    buf.put_u32(id); // 4
    buf.put_u16(proto_id); // 2
    buf.put_u8(reqid); // 1
    buf.put_u8(uid.len() as u8); // 1
    buf.put_slice(uid.as_bytes());
    buf.put_u32(svrid); // 4
    buf.put(data);
    buf.freeze()
}

pub fn decode(mut bytes: Bytes) -> (u32, u16, u8, String, u32, Bytes) {
    let id = bytes.get_u32();
    let proto_id = bytes.get_u16();
    let reqid = bytes.get_u8();
    let uid_len = bytes.get_u8();
    let uid = String::from_utf8(bytes.slice(0..uid_len as usize).to_vec()).unwrap();
    bytes.advance(uid_len as usize);
    let svrid = bytes.get_u32();
    (id, proto_id, reqid, uid, svrid, bytes)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let id = 123;
        let proto_id = 456;
        let reqid = 7;
        let uid = String::from("test");
        let svrid = 789;
        let data = Bytes::from("payload");

        let encoded = encode(id, proto_id, reqid, &uid, svrid, data.clone());
        let (decoded_id, decoded_proto_id, decoded_reqid, decoded_uid, decoded_svrid, decoded_data) =
            decode(encoded);

        assert_eq!(decoded_id, id);
        assert_eq!(decoded_proto_id, proto_id);
        assert_eq!(decoded_reqid, reqid);
        assert_eq!(decoded_uid, uid);
        assert_eq!(decoded_svrid, svrid);
        assert_eq!(decoded_data, data);
    }

    #[test]
    fn test_encode_decode_empty_data() {
        let id = 123;
        let proto_id = 456;
        let reqid = 7;
        let uid = String::from("test");
        let svrid = 789;
        let data = Bytes::new();

        let encoded = encode(id, proto_id, reqid, &uid, svrid, data.clone());
        let (decoded_id, decoded_proto_id, reqid, decoded_uid, decoded_svrid, decoded_data) =
            decode(encoded);

        assert_eq!(decoded_id, id);
        assert_eq!(decoded_proto_id, proto_id);
        assert_eq!(reqid, reqid);
        assert_eq!(decoded_uid, uid);
        assert_eq!(decoded_svrid, svrid);
        assert_eq!(decoded_data, data);
    }

    #[test]
    fn test_encode_decode_long_uid() {
        let id = 123;
        let proto_id = 456;
        let reqid = 7;
        let uid = String::from("this_is_a_very_long_uid_that_exceeds_the_capacity_of_a_u8");
        let svrid = 789;
        let data = Bytes::from("payload");

        let encoded = encode(id, proto_id, reqid, &uid, svrid, data.clone());
        let (decoded_id, decoded_proto_id, reqid, decoded_uid, decoded_svrid, decoded_data) =
            decode(encoded);

        assert_eq!(decoded_id, id);
        assert_eq!(decoded_proto_id, proto_id);
        assert_eq!(reqid, reqid);
        assert_eq!(decoded_uid, uid);
        assert_eq!(decoded_svrid, svrid);
        assert_eq!(decoded_data, data);
    }
}
