use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use esp_hal::{
    clock::Clocks,
    gpio::{AnyPin, InputOutputPinType, Unknown},
    i2c::I2C,
    peripherals::I2C0,
};
use fugit::RateExtU32;
use static_cell::StaticCell;

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, I2C<I2C0>>> = StaticCell::new();

pub fn init(
    i2c: I2C0,
    clocks: &Clocks,
    clock: AnyPin<Unknown, InputOutputPinType>,
    sda: AnyPin<Unknown, InputOutputPinType>,
) -> &'static mut Mutex<NoopRawMutex, I2C<'static, I2C0>> {
    let i2c = I2C::new(i2c, clock, sda, 800_u32.kHz(), &clocks);

    let i2c_bus = Mutex::new(i2c);

    I2C_BUS.init(i2c_bus)
}
