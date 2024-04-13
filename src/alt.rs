use bme280::i2c::AsyncBME280;
use embassy_executor::task;
use embassy_time::{Delay, Timer};
use esp_hal::{i2c::I2C, peripherals::I2C0};
use micromath::F32Ext;

use crate::STATE;

fn get_absolute_altitude_from_pressure(pressure: f32) -> f32 {
    // TODO This isn't giving me the numbers I expect
    const SEA_LEVEL_PRESSURE_HPA: f32 = 101325_f32;
    44_330_f32 * (1_f32 - f32::powf(pressure / SEA_LEVEL_PRESSURE_HPA, 0.1903_f32))
}

#[task]
pub async fn sample(i2c: I2C<'static, I2C0>) -> ! {
    let mut alitmeter = AsyncBME280::new_primary(i2c);
    alitmeter.init(&mut Delay).await.unwrap();

    loop {
        let current_alt = alitmeter.measure(&mut Delay).await.unwrap();
        {
            let mut state = STATE.lock().await;
            state.altimeter_altitude = get_absolute_altitude_from_pressure(current_alt.pressure);
        }

        Timer::after_millis(1_000).await;
    }
}
