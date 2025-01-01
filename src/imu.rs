// use bno055::BNO055OperationMode;
// use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
// use embassy_executor::task;
// use embassy_sync::blocking_mutex::raw::NoopRawMutex;
// use embassy_time::{Delay, Timer};
// use esp_hal::{i2c::I2C, peripherals::I2C0, Async};

// #[task]
// pub async fn sample(i2c: I2cDevice<'static, NoopRawMutex, I2C<'static, I2C0, Async>>) -> ! {
//     Timer::after_millis(2_000).await;

//     let mut imu = bno055::Bno055::new(i2c);

//     imu.init(&mut Delay).await.unwrap();

//     // Set to 9DOF Absolute Sensor Fusion mode
//     imu.set_mode(BNO055OperationMode::NDOF, &mut Delay)
//         .await
//         .unwrap();

//     // Update the G range of the accelerometer to ±16G
//     let mut acc_config = imu.get_acc_config().await.unwrap();
//     acc_config.set_g_range(bno055::AccGRange::G16);
//     imu.set_acc_config(&acc_config).await.unwrap();

//     // This is annoyingly blocking
//     while !imu.is_fully_calibrated().await.unwrap() {
//         Timer::after_millis(100).await;
//     }

//     loop {
//         // The quaternion encodes the info from the gyroscope
//         let _q = imu.quaternion().await.unwrap();

//         // The linear acceleration encodes the info from the accelerometer
//         // Gavity should be removed due to the sensor fusion mode
//         let _l = imu.linear_acceleration().await.unwrap();

//         Timer::after_millis(1_000).await;
//     }
// }
