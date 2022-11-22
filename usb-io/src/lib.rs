#![cfg_attr(not(feature = "std"), no_std)]

pub mod class;

#[cfg(feature = "std")]
pub mod host;

use serde::{Deserialize, Serialize};

pub const VID: u16 = 0x16c0;
pub const PID: u16 = 0x27dd;
pub const MANUFACTURER: &'static str = "USB-IO Manafacturer";
pub const PRODUCT: &'static str = "USB-IO USB class";
pub const SERIAL_NUMBER: &'static str = "USB-IO Serial Number";
pub const MESSAGE_MAX_SIZE: u16 = 16;

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Message {
    Ping,
    Pong,
}

#[cfg(test)]
mod test {
    use super::*;
    use postcard::{from_bytes, to_slice};

    #[test]
    fn test_encode_decode_message() {
        let message = Message::Ping;
        let mut buf = [0u8; MESSAGE_MAX_SIZE as usize];
        let slice = to_slice(&message, &mut buf).unwrap();
        assert_eq!(message, from_bytes(&slice).unwrap());
    }
}
