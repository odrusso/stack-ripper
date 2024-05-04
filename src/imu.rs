use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::task;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Delay, Timer};
use esp_hal::{i2c::I2C, peripherals::I2C0};

// use crate::state::STATE;

#[task]
pub async fn sample(i2c: I2cDevice<'static, NoopRawMutex, I2C<'static, I2C0>>) -> ! {
    Timer::after_millis(2_000).await;

    let mut imu = bno055::Bno055::new(i2c);

    imu.init(&mut Delay).await.unwrap();

    loop {
        Timer::after_millis(1_000).await;
    }
}
