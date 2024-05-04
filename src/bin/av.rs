#![deny(unsafe_code)]
#![no_main]
#![no_std]


use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::{task, Spawner};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::Timer;
use static_cell::StaticCell;

use esp_hal::{
    clock::{ClockControl, CpuClock}, embassy, i2c::I2C, peripherals::{Peripherals, I2C0}, prelude::*, timer::TimerGroup, IO
};

use defmt::info;
use esp_backtrace as _;

use stack_ripper::{alt, imu, state};

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, I2C<I2C0>>> = StaticCell::new();


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

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();
    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    // Setup I2C bus
    let i2c_clock = io.pins.gpio8;
    let i2c_data = io.pins.gpio9;

    let i2c = I2C::new(
        peripherals.I2C0,
        i2c_clock,
        i2c_data,
        800_u32.kHz(),
        &clocks);

    let i2c_bus = Mutex::new(i2c);
    let i2c_bus = I2C_BUS.init(i2c_bus);

    let i2c_alt = I2cDevice::new(i2c_bus);
    _spawner.spawn(alt::sample(i2c_alt)).ok();

    let i2c_imu = I2cDevice::new(i2c_bus);
    _spawner.spawn(imu::sample(i2c_imu)).ok();

    info!("Initializing compete");

    // Finally set up the task to print state
    _spawner.spawn(print_state()).ok();
}
