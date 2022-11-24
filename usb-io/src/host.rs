mod connection;
mod device;

pub use self::{
    connection::Connection,
    device::{Device, Devices},
};

use std::time::Duration;

pub const TIMEOUT: Duration = Duration::from_secs(1);
