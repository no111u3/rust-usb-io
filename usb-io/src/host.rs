use postcard::{from_bytes, to_slice};
use rusb::{Context, DeviceHandle, UsbContext as _};

use std::{
    fmt::{self, Debug},
    slice::Iter,
    sync::Mutex,
    time::Duration,
    vec::IntoIter,
};

use crate::{Message, MESSAGE_MAX_SIZE, PID, USB_IO_IN_ENDPOINT, USB_IO_OUT_ENDPOINT, VID};

pub const TIMEOUT: Duration = Duration::from_secs(1);
pub const EN_US: u16 = 0x0409;

/// Number of times to retry a bulk message receive operation before giving up
const MAX_RECV_RETRIES: usize = 3;

pub struct Devices(Vec<Device>);

impl Devices {
    pub fn detect(timeout: Duration) -> Result<Self, rusb::Error> {
        let device_list = Context::new()?.devices()?;
        let mut devices = vec![];
        println!("USB: enumerating devices...");

        for device in device_list.iter() {
            let desc = device.device_descriptor()?;

            if desc.vendor_id() != VID || desc.product_id() != PID {
                continue;
            }

            println!("found USB-IO device: {:?}", device);

            let mut handle = device.open()?;

            handle.reset()?;

            let language = *handle.read_languages(timeout)?.first().unwrap();

            let t = timeout;
            let manufacturer = handle.read_manufacturer_string(language, &desc, t)?;
            let product = handle.read_product_string(language, &desc, t)?;
            let product_name = format!("{} {}", manufacturer, product);
            let serial_number = handle.read_serial_number_string(language, &desc, t)?;

            println!(
                "USB(bus={},addr={}): found {} (serial #{})",
                device.bus_number(),
                device.address(),
                product_name,
                serial_number,
            );

            devices.push(Device::new(device, product_name, serial_number));
        }

        if devices.is_empty() {
            println!("no USB-IO devices found");
        }

        Ok(Self(devices))
    }

    /// Number of detected devices
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Has no one of these
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Borrow the detected devices as a slice
    pub fn as_slice(&self) -> &[Device] {
        self.0.as_slice()
    }

    /// Iterate over the detected IO-USB devices
    pub fn iter(&self) -> Iter<'_, Device> {
        self.0.iter()
    }
}

impl IntoIterator for Devices {
    type Item = Device;
    type IntoIter = IntoIter<Device>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

pub struct Device {
    /// Underlying `rusb` device
    pub(super) device: rusb::Device<rusb::Context>,

    /// Product vendor and name
    pub product_name: String,

    /// Serial number
    pub serial_number: String,
}

impl Device {
    /// Create a new device
    pub(super) fn new(
        device: rusb::Device<rusb::Context>,
        product_name: String,
        serial_number: String,
    ) -> Self {
        Self {
            device,
            product_name,
            serial_number,
        }
    }

    /// Open this device, consuming it and creating a `UsbConnection`
    pub fn open(self, timeout: Duration) -> Result<UsbConnection, rusb::Error> {
        let connection = UsbConnection::create(self, timeout)?;

        println!(
            "USB(bus={},addr={}): successfully opened {} (serial #{})",
            connection.device().bus_number(),
            connection.device().address(),
            connection.device().product_name,
            connection.device().serial_number,
        );

        Ok(connection)
    }

    /// Get the bus number for this device
    pub fn bus_number(&self) -> u8 {
        self.device.bus_number()
    }

    /// Get the address for this device
    pub fn address(&self) -> u8 {
        self.device.address()
    }

    /// Open a handle to the underlying device (for use by `UsbConnection`)
    pub(super) fn open_handle(&self) -> Result<rusb::DeviceHandle<rusb::Context>, rusb::Error> {
        let mut handle = self.device.open()?;
        handle.reset()?;
        handle.claim_interface(0)?;

        // Flush any unconsumed messages still in the buffer
        //flush(&mut handle)?;

        Ok(handle)
    }
}

impl Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Device(bus={} addr={} serial=#{})",
            self.bus_number(),
            self.address(),
            self.serial_number,
        )
    }
}

/// Connection to USB-IO via USB
pub struct UsbConnection {
    /// Handle to the underlying USB device
    handle: Mutex<DeviceHandle<Context>>,

    /// USB-IO device this connection is connected to
    device: Device,

    /// Timeout for reading from / writing to the USB-IO
    timeout: Duration,
}

impl UsbConnection {
    /// Create a new YubiHSM device from a rusb device
    pub(super) fn create(device: Device, timeout: Duration) -> Result<Self, rusb::Error> {
        let handle = device.open_handle()?;

        // Clear any lingering messages
        /*for _ in 0..MAX_RECV_RETRIES {
            if recv_message(&handle, UsbTimeout::from_millis(1)).is_err() {
                break;
            }
        }*/

        Ok(Self {
            device,
            timeout,
            handle: Mutex::new(handle),
        })
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
}
