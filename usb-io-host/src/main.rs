use usb_io::{
    host::{Devices, TIMEOUT},
    Message,
};

fn main() {
    let devices = Devices::detect(TIMEOUT).unwrap();
    for device in devices.into_iter() {
        println!("{:?}", device);
        if let Ok(mut connection) = device.open(TIMEOUT) {
            if connection.send_message(Message::Ping).is_ok() {
                if let Ok(message) = connection.recv_message() {
                    println!("{:?}", message);
                } else {
                    println!("could not receive");
                }
            }
        }
    }
}
