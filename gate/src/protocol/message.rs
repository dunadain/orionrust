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
    let mut offset = 1;
    let msg_type = bytes.get_u8();
    let id = match get_msg_type(msg_type) {
        MsgType::Request | MsgType::Response => bytes.get_u8(),
        _ => 0,
    };
    if id > 0 {
        offset += 1;
    }
    let protocol_id = match get_msg_type(msg_type) {
        MsgType::Request | MsgType::Notify | MsgType::Push => bytes.get_u16(),
        _ => 0,
    };
    if protocol_id > 0 {
        offset += 2;
    }

    (
        get_msg_type(msg_type),
        protocol_id,
        id,
        bytes.slice(offset..),
    )
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
