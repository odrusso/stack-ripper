use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use esp_hal::{clock::Clocks, gpio::any_pin::AnyPin, i2c::I2C, peripherals::I2C0, Async};
use fugit::RateExtU32;
use static_cell::StaticCell;

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, I2C<I2C0, Async>>> = StaticCell::new();

pub fn init(
    i2c: I2C0,
    clocks: &Clocks,
    clock: AnyPin<'static>,
    sda: AnyPin<'static>,
) -> &'static mut Mutex<NoopRawMutex, I2C<'static, I2C0, Async>> {
    let i2c = I2C::new_async(i2c, sda, clock, 800_u32.kHz(), &clocks);

    let i2c_bus = Mutex::new(i2c);

    I2C_BUS.init(i2c_bus)
}
