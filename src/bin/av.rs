#![deny(unsafe_code)]
#![no_main]
#![no_std]

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::{task, Spawner};
use embassy_time::Timer;

use esp_hal::{
    clock::{ClockControl, CpuClock},
    gpio::{any_pin::AnyPin, Io},
    peripherals::Peripherals,
    prelude::*,
    system::SystemControl,
    timer::timg::TimerGroup,
};

use defmt::info;
use esp_backtrace as _;

use stack_ripper::{alt, i2c, imu, state};

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

    let peripherals: Peripherals = Peripherals::take();
    let system = SystemControl::new(peripherals.SYSTEM);
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();
    let timg0 = TimerGroup::new_async(peripherals.TIMG0, &clocks);

    esp_hal_embassy::init(&clocks, timg0);

    let io = Io::new(peripherals.GPIO, peripherals.IO_MUX);

    info!("Initializing compete");

    // Setup I2C bus
    let i2c_clock = AnyPin::new(io.pins.gpio8);
    let i2c_data = AnyPin::new(io.pins.gpio9);

    let i2c_bus = i2c::init(peripherals.I2C0, &clocks, i2c_clock, i2c_data);

    // let i2c_alt = I2cDevice::new(i2c_bus);
    // _spawner.spawn(alt::sample(i2c_alt)).ok();

    let i2c_imu = I2cDevice::new(i2c_bus);
    _spawner.spawn(imu::sample(i2c_imu)).ok();

    info!("Initializing compete");

    // Finally set up the task to print state
    _spawner.spawn(print_state()).ok();
}
