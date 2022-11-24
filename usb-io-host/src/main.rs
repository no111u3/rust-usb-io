use std::thread::sleep;
use std::time::Duration;
use usb_io::{
    host::{Devices, TIMEOUT},
    InfallibleMemoryInterface,
};

fn main() {
    let devices = Devices::detect(TIMEOUT).unwrap();
    for device in devices.into_iter() {
        println!("{:?}", device);
        if let Ok(connection) = device.open(TIMEOUT) {
            if connection.ready_to_use() {
                println!(
                    "USB IO board with serial {} ready to use",
                    connection.device().serial_number
                );
                let base_gpioc_addr: u32 = 1073874944;
                let base_rcc_addr: u32 = 1073887232;
                let mut reg_value = connection.read32(base_rcc_addr + 48);
                reg_value |= 1 << 2;
                connection.write32(base_rcc_addr + 48, reg_value);

                let moder_value = 1 << 26;
                let mut data: u32 = 1 << 13;
                connection.write32(base_gpioc_addr, moder_value);
                for _ in 0..5 {
                    connection.write32(base_gpioc_addr + 20, data);
                    data ^= 1 << 13;
                    sleep(Duration::from_millis(500));
                }
            }
        }
    }
}
