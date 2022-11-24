use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Data {
    U8(u8),
    U16(u16),
    U32(u32),
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum DataSize {
    U8,
    U16,
    U32,
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Message {
    /// Ping ask
    Ping,
    /// Ping answer
    Pong,
    /// Acknow response
    Ack,
    /// Data response
    Data(Data),
    /// Set address to data
    Set(u32, Data),
    /// Get from address to data size
    Get(u32, DataSize),
    /// No operation (used for coverage testing and performance metrics)
    Nop,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::usb::MESSAGE_MAX_SIZE;
    use postcard::{from_bytes, to_slice};

    #[test]
    fn test_encode_decode_message() {
        let message = Message::Ping;
        let mut buf = [0u8; MESSAGE_MAX_SIZE as usize];
        let slice = to_slice(&message, &mut buf).unwrap();
        assert_eq!(message, from_bytes(&slice).unwrap());
    }
}
