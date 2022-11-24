use crate::host::Connection;

use rusb::{Context, UsbContext as _};

use std::{
    fmt::{self, Debug},
    slice::Iter,
    time::Duration,
    vec::IntoIter,
};

use crate::usb::{MESSAGE_MAX_SIZE, PID, USB_IO_IN_ENDPOINT, VID};

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
    pub fn open(self, timeout: Duration) -> Result<Connection, rusb::Error> {
        let connection = Connection::create(self, timeout)?;

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
        Self::flush(&mut handle)?;

        Ok(handle)
    }

    /// Flush any unconsumed messages still in the buffer to get the connection
    /// back into a clean state
    fn flush(handle: &mut rusb::DeviceHandle<rusb::Context>) -> Result<(), rusb::Error> {
        let mut buffer = [0u8; MESSAGE_MAX_SIZE as usize];

        // Use a near instantaneous (but non-zero) timeout to drain the buffer.
        // Zero is interpreted as wait forever.
        let timeout = Duration::from_millis(1);

        match handle.read_bulk(USB_IO_IN_ENDPOINT, &mut buffer, timeout) {
            Ok(_) | Err(rusb::Error::Timeout) => Ok(()),
            Err(e) => Err(e.into()),
        }
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
