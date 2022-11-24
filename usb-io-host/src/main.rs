use std::thread::sleep;
use std::time::Duration;
use usb_io::{
    host::{Devices, TIMEOUT},
    message::{Data, DataSize, Message},
};

fn main() {
    let devices = Devices::detect(TIMEOUT).unwrap();
    for device in devices.into_iter() {
        println!("{:?}", device);
        if let Ok(connection) = device.open(TIMEOUT) {
            if let Ok(Message::Pong) = connection.request(Message::Ping) {
                println!(
                    "USB IO board with serial {} ready to use",
                    connection.device().serial_number
                );
                let base_gpioc_addr: u32 = 1073874944;
                let base_rcc_addr: u32 = 1073887232;
                let message = connection
                    .request(Message::Get(base_rcc_addr + 48, DataSize::U32))
                    .unwrap();
                if let Message::Data(Data::U32(mut reg_value)) = message {
                    reg_value |= 1 << 2;
                    connection
                        .request(Message::Set(base_rcc_addr + 48, Data::U32(reg_value)))
                        .unwrap();
                }

                let moder_value = 1 << 26;
                let mut data: u32 = 1 << 13;
                connection
                    .request(Message::Set(base_gpioc_addr, Data::U32(moder_value)))
                    .unwrap();
                for _ in 0..5 {
                    connection
                        .request(Message::Set(base_gpioc_addr + 20, Data::U32(data)))
                        .unwrap();
                    data ^= 1 << 13;
                    sleep(Duration::from_millis(500));
                }
            }
        }
    }
}
