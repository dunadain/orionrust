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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode() {
        let data = Bytes::from("hello");
        let pkt = encode(PacketType::Data, data);
        assert_eq!(pkt.len(), 9);
        assert_eq!(pkt[0], 3);
        assert_eq!(pkt[1], 0);
        assert_eq!(pkt[2], 0);
        assert_eq!(pkt[3], 5);
        assert_eq!(&pkt[4..], b"hello");
    }

    #[test]
    fn test_decode() {
        let data = Bytes::from("hello");
        let pkt = encode(PacketType::Data, data);
        let (pkt_type, data) = decode(pkt);
        assert_eq!(pkt_type as u8, PacketType::Data as u8);
        assert_eq!(data, Bytes::from("hello"));
    }
}
