use bytes::{BufMut, Bytes, BytesMut};

const PKT_HEAD_LEN: usize = 4;

pub enum PacketType {
    Handshake,
    HandshakeAck,
    Heartbeat,
    Data,
    Kick,
    Error,
}

pub fn encode(pkt_type: PacketType, bytes: Bytes) -> Bytes {
    let length = bytes.len();
    let mut buf = BytesMut::with_capacity(PKT_HEAD_LEN + length);
    buf.put_u8(pkt_type as u8);
    buf.put_u8((length >> 16) as u8);
    buf.put_u8((length >> 8) as u8);
    buf.put_u8(length as u8);
    buf.extend_from_slice(&bytes);
    buf.freeze()
}

pub fn decode(bytes: Bytes) -> (PacketType, Bytes) {
    let pkt_type = bytes[0];
    // let length = (bytes[1] as usize) << 16 | (bytes[2] as usize) << 8 | bytes[3] as usize;
    let data = bytes.slice(PKT_HEAD_LEN..);
    (get_pkt_type(pkt_type), data)
}

fn get_pkt_type(pkt_type: u8) -> PacketType {
    match pkt_type {
        0 => PacketType::Handshake,
        1 => PacketType::HandshakeAck,
        2 => PacketType::Heartbeat,
        3 => PacketType::Data,
        4 => PacketType::Kick,
        5 => PacketType::Error,
        _ => panic!("Invalid packet type"),
    }
}
