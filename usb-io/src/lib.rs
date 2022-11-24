#![cfg_attr(not(feature = "std"), no_std)]

pub mod class;

#[cfg(feature = "std")]
pub mod host;

pub mod memory_interface;

pub mod message;

pub mod usb;

pub use memory_interface::{InfallibleMemoryInterface, MemoryInterface};
