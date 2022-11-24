use postcard::{from_bytes, to_slice};
use std::{sync::Mutex, time::Duration};

use rusb::{Context, DeviceHandle};

use crate::{
    host::Device,
    memory_interface::MemoryInterface,
    message::{Data, DataSize, Message},
    usb::{MESSAGE_MAX_SIZE, USB_IO_IN_ENDPOINT, USB_IO_OUT_ENDPOINT},
};

/// Number of times to retry a bulk message receive operation before giving up
const MAX_RECV_RETRIES: usize = 3;

/// Connection to USB-IO via USB
pub struct Connection {
    /// Handle to the underlying USB device
    handle: Mutex<DeviceHandle<Context>>,

    /// USB-IO device this connection is connected to
    device: Device,

    /// Timeout for reading from / writing to the USB-IO
    timeout: Duration,
}

impl Connection {
    /// Create a new YubiHSM device from a rusb device
    pub(super) fn create(device: Device, timeout: Duration) -> Result<Self, rusb::Error> {
        let handle = device.open_handle()?;

        let connection = Self {
            device,
            timeout,
            handle: Mutex::new(handle),
        };

        // Clear any lingering messages
        for _ in 0..MAX_RECV_RETRIES {
            if connection.recv_message().is_err() {
                break;
            }
        }

        Ok(connection)
    }

    /// Borrow the `Device` for this connection
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Write a bulk message to the USB-IO
    pub fn send_message(&self, message: Message) -> Result<usize, rusb::Error> {
        let mut buf = [0; MESSAGE_MAX_SIZE as usize];
        let slice = to_slice(&message, &mut buf).unwrap();
        let nbytes =
            self.handle
                .lock()
                .unwrap()
                .write_bulk(USB_IO_OUT_ENDPOINT, slice, self.timeout)?;

        if slice.len() == nbytes {
            Ok(nbytes)
        } else {
            Err(rusb::Error::Other)
        }
    }

    /// Receive a message
    pub fn recv_message(&self) -> Result<Message, rusb::Error> {
        // Allocate a buffer which is the maximum size we expect to receive
        let mut buf = [0; MESSAGE_MAX_SIZE as usize];

        for attempts_remaining in (0..MAX_RECV_RETRIES).rev() {
            match self
                .handle
                .lock()
                .unwrap()
                .read_bulk(USB_IO_IN_ENDPOINT, &mut buf, self.timeout)
            {
                Ok(_) => {
                    if let Ok(message) = from_bytes(&buf) {
                        return Ok(message);
                    } else {
                        return Err(rusb::Error::Other);
                    }
                }

                // Sometimes I/O errors occur sporadically. When this happens,
                // retry the read for `MAX_RECV_RETRIES` attempts
                Err(rusb::Error::Io) => {
                    println!(
                        "I/O error during USB bulk message receive, retrying ({} attempts remaining)",
                        attempts_remaining
                    );
                    continue;
                }
                // All other errors we return immediately
                Err(err) => return Err(err),
            }
        }
        Err(rusb::Error::Other)
    }

    pub fn request(&self, message: Message) -> Result<Message, rusb::Error> {
        self.send_message(message)?;
        self.recv_message()
    }

    pub fn ready_to_use(&self) -> bool {
        if let Ok(Message::Pong) = self.request(Message::Ping) {
            true
        } else {
            false
        }
    }
}

impl MemoryInterface for Connection {
    type Error = rusb::Error;

    fn try_read8(&self, address: u32) -> Result<u8, Self::Error> {
        let data = self.request(Message::Get(address, DataSize::U8))?;
        if let Message::Data(Data::U8(data)) = data {
            Ok(data)
        } else {
            Err(rusb::Error::Other)
        }
    }

    fn try_read16(&self, address: u32) -> Result<u16, Self::Error> {
        let data = self.request(Message::Get(address, DataSize::U16))?;
        if let Message::Data(Data::U16(data)) = data {
            Ok(data)
        } else {
            Err(rusb::Error::Other)
        }
    }

    fn try_read32(&self, address: u32) -> Result<u32, Self::Error> {
        let data = self.request(Message::Get(address, DataSize::U32))?;
        if let Message::Data(Data::U32(data)) = data {
            Ok(data)
        } else {
            Err(rusb::Error::Other)
        }
    }

    fn try_write8(&self, address: u32, value: u8) -> Result<(), Self::Error> {
        self.request(Message::Set(address, Data::U8(value)))?;
        Ok(())
    }

    fn try_write16(&self, address: u32, value: u16) -> Result<(), Self::Error> {
        self.request(Message::Set(address, Data::U16(value)))?;
        Ok(())
    }

    fn try_write32(&self, address: u32, value: u32) -> Result<(), Self::Error> {
        self.request(Message::Set(address, Data::U32(value)))?;
        Ok(())
    }
}
