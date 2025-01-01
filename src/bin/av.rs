#![deny(unsafe_code)]
#![no_main]
#![no_std]

use embassy_executor::{task, Spawner};
use embassy_time::Timer;

use esp_hal::{
    peripherals::Peripherals,
    prelude::*,
    timer::timg::TimerGroup,
};

use defmt::info;
use esp_backtrace as _;

use stack_ripper::state;

#[task]
async fn print_state() -> ! {
    loop {
        info!("{:?}", *state::STATE.lock().await);
        Timer::after_millis(5_000).await;
    }
}

#[main]
async fn main(_spawner: Spawner) -> () {
    info!("Initializing");

    let peripherals: Peripherals = esp_hal::init(esp_hal::Config::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);

    esp_hal_embassy::init(timg0.timer0);

    info!("Initializing compete");

    // // Setup I2C bus
    // let i2c_clock = AnyPin::new(io.pins.gpio8);
    // let i2c_data = AnyPin::new(io.pins.gpio9);

    // let i2c_bus = i2c::init(peripherals.I2C0, &clocks, i2c_clock, i2c_data);

    // // let i2c_alt = I2cDevice::new(i2c_bus);
    // // _spawner.spawn(alt::sample(i2c_alt)).ok();

    // let i2c_imu = I2cDevice::new(i2c_bus);
    // _spawner.spawn(imu::sample(i2c_imu)).ok();

    // info!("Initializing compete");

    // Finally set up the task to print state
    _spawner.spawn(print_state()).ok();
}
