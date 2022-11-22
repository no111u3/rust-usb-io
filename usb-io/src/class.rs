use core::marker::PhantomData;
use postcard::{from_bytes, to_slice};
use usb_device::class_prelude::*;
use usb_device::device::{UsbDevice, UsbDeviceBuilder, UsbVidPid};
use usb_device::Result;

use crate::{Message, MANUFACTURER, MESSAGE_MAX_SIZE, PID, PRODUCT, SERIAL_NUMBER, VID};

pub struct UsbIoClass<'a, B: UsbBus> {
    interface: InterfaceNumber,
    read_ep: EndpointOut<'a, B>,
    write_ep: EndpointIn<'a, B>,
    _marker: PhantomData<B>,
}

impl<B: UsbBus> UsbIoClass<'_, B> {
    pub fn new(alloc: &UsbBusAllocator<B>) -> UsbIoClass<B> {
        UsbIoClass {
            interface: alloc.interface(),
            read_ep: alloc.bulk(MESSAGE_MAX_SIZE),
            write_ep: alloc.bulk(MESSAGE_MAX_SIZE),
            _marker: PhantomData,
        }
    }

    pub fn make_device<'a, 'b>(
        &'a self,
        usb_bus: &'b UsbBusAllocator<B>,
        serial: Option<&'static str>,
    ) -> UsbDevice<'b, B> {
        let serial = serial.unwrap_or(SERIAL_NUMBER);
        UsbDeviceBuilder::new(&usb_bus, UsbVidPid(VID, PID))
            .manufacturer(MANUFACTURER)
            .product(PRODUCT)
            .serial_number(serial)
            .build()
    }
}

impl<B: UsbBus> UsbClass<B> for UsbIoClass<'_, B> {
    fn get_configuration_descriptors(&self, writer: &mut DescriptorWriter) -> Result<()> {
        writer.interface(self.interface, 0xff, 0, 0)?;
        writer.endpoint(&self.write_ep)?;
        writer.endpoint(&self.read_ep)?;
        Ok(())
    }

    fn endpoint_out(&mut self, addr: EndpointAddress) {
        if addr == self.read_ep.address() {
            let mut buf = [0; MESSAGE_MAX_SIZE as usize];
            let size = self.read_ep.read(&mut buf).unwrap();

            self.read_ep.stall();

            if size < 1 {
                return;
            }

            if let Ok(message) = from_bytes(&buf) {
                let return_message = match message {
                    Message::Ping => Message::Pong,
                    Message::Pong => Message::Ping,
                };

                let slice = to_slice(&return_message, &mut buf).unwrap();

                self.write_ep.write(&slice).unwrap();
            } else {
                return;
            }
        }
    }

    fn endpoint_in_complete(&mut self, addr: EndpointAddress) {
        if addr == self.write_ep.address() {
            self.read_ep.unstall();
        }
    }
}
