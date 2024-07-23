use bytes::{Buf, BufMut, Bytes, BytesMut};

pub enum MsgType {
    Request,
    Response,
    Notify,
    Push,
}

const MSG_TYPE_LEN: usize = 1;
const MSG_PROTOCOL_ID_LEN: usize = 2;

pub fn encode(msg_type: MsgType, protocol_id: u16, id: u8, data: Bytes) -> bytes::Bytes {
    let id_len = match msg_type {
        MsgType::Request | MsgType::Response => 1,
        _ => 0,
    };
    let mut msg_len = id_len + MSG_TYPE_LEN;
    let proto_len = match msg_type {
        MsgType::Request | MsgType::Notify | MsgType::Push => MSG_PROTOCOL_ID_LEN,
        _ => 0,
    };
    msg_len += proto_len;
    msg_len += data.len();
    let mut buf = BytesMut::with_capacity(msg_len);
    buf.put_u8(msg_type as u8);
    if id_len > 0 {
        buf.put_u8(id);
    }
    if proto_len > 0 {
        buf.put_u16(protocol_id);
    }
    buf.extend_from_slice(&data);
    buf.freeze()
}

pub fn decode(mut bytes: Bytes) -> (MsgType, u16, u8, Bytes) {
    let msg_type = bytes.get_u8();
    let id = match get_msg_type(msg_type) {
        MsgType::Request | MsgType::Response => bytes.get_u8(),
        _ => 0,
    };
    let protocol_id = match get_msg_type(msg_type) {
        MsgType::Request | MsgType::Notify | MsgType::Push => bytes.get_u16(),
        _ => 0,
    };

    (get_msg_type(msg_type), protocol_id, id, bytes)
}

fn get_msg_type(msg_type: u8) -> MsgType {
    match msg_type {
        0 => MsgType::Request,
        1 => MsgType::Response,
        2 => MsgType::Notify,
        3 => MsgType::Push,
        _ => panic!("Invalid message type"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_request() {
        let msg_type = MsgType::Request;
        let protocol_id = 1234;
        let id = 5;
        let data = Bytes::from("Hello, world!");

        let encoded = encode(msg_type, protocol_id, id, data.clone());

        let expected_len = 1 + 1 + 2 + data.len();
        assert_eq!(encoded.len(), expected_len);

        let decoded = decode(encoded);
        assert_eq!(decoded.0 as u8, MsgType::Request as u8);
        assert_eq!(decoded.1, protocol_id);
        assert_eq!(decoded.2, id);
        assert_eq!(decoded.3, data);
    }

    #[test]
    fn test_encode_response() {
        let msg_type = MsgType::Response;
        let protocol_id = 5678;
        let id = 9;
        let data = Bytes::from("Hello, GitHub Copilot!");

        let encoded = encode(msg_type, protocol_id, id, data.clone());

        let expected_len = 1 + 1 + data.len();
        assert_eq!(encoded.len(), expected_len);

        let decoded = decode(encoded);
        assert_eq!(decoded.0 as u8, MsgType::Response as u8);
        assert_eq!(decoded.1, 0);
        assert_eq!(decoded.2, id);
        assert_eq!(decoded.3, data);
    }

    #[test]
    fn test_encode_notify() {
        let msg_type = MsgType::Notify;
        let protocol_id = 9876;
        let id = 0;
        let data = Bytes::from("Hello from the server!");

        let encoded = encode(msg_type, protocol_id, id, data.clone());

        let expected_len = 1 + 2 + data.len();
        assert_eq!(encoded.len(), expected_len);

        let decoded = decode(encoded);
        assert_eq!(decoded.0 as u8, MsgType::Notify as u8);
        assert_eq!(decoded.1, protocol_id);
        assert_eq!(decoded.2, id);
        assert_eq!(decoded.3, data);
    }

    #[test]
    fn test_encode_push() {
        let msg_type = MsgType::Push;
        let protocol_id = 4321;
        let id = 0;
        let data = Bytes::from("Pushing updates...");

        let encoded = encode(msg_type, protocol_id, id, data.clone());

        let expected_len = 1 + 2 + data.len();
        assert_eq!(encoded.len(), expected_len);

        let decoded = decode(encoded);
        assert_eq!(decoded.0 as u8, MsgType::Push as u8);
        assert_eq!(decoded.1, protocol_id);
        assert_eq!(decoded.2, id);
        assert_eq!(decoded.3, data);
    }
}
