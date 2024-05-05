#![deny(unsafe_code)]
#![no_main]
#![no_std]

pub mod i2c;
pub mod spi;

pub mod alt;
pub mod gps;
pub mod imu;
pub mod lora;
pub mod state;
