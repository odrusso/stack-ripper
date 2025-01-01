#![deny(unsafe_code)]
#![no_main]
#![no_std]

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::{task, Spawner};
use embassy_time::Timer;

use esp_hal::{
    clock::{ClockControl, CpuClock},
    gpio::{any_pin::AnyPin, AnyInput, AnyOutput, Io, Level, Pull},
    peripherals::Peripherals,
    prelude::*,
    system::SystemControl,
    timer::timg::TimerGroup,
};

use defmt::info;
use esp_backtrace as _;

use stack_ripper::{lora, spi, state};

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

    // Set SPI bus
    let spi_clock = AnyPin::new(io.pins.gpio4);
    let spi_miso = AnyPin::new(io.pins.gpio3);
    let spi_mosi = AnyPin::new(io.pins.gpio2);

    let spi_bus = spi::init(
        peripherals.DMA,
        peripherals.SPI2,
        &clocks,
        spi_clock,
        spi_mosi,
        spi_miso,
    );

    let lora_spi_csb = AnyOutput::new(io.pins.gpio1, Level::High);
    let lora_spi = SpiDevice::new(spi_bus, lora_spi_csb.into());

    let lora_rst: AnyOutput<'_> = AnyOutput::new(io.pins.gpio6, Level::High);
    let lora_irq = AnyInput::new(io.pins.gpio5, Pull::Up);

    _spawner
        .spawn(lora::receive(lora_spi, lora_irq.into(), lora_rst.into()))
        .ok();
}
