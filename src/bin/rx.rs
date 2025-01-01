#![deny(unsafe_code)]
#![no_main]
#![no_std]

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_hal::{
    gpio::{Input, Level, Output, Pin, Pull},
    peripherals::Peripherals,
    timer::timg::TimerGroup,
};
use defmt::info;
use esp_backtrace as _;

use stack_ripper::{lora, spi, state};

#[embassy_executor::task]
async fn print_state() -> ! {
    loop {
        info!("{:?}", *state::STATE.lock().await);
        Timer::after_millis(5_000).await;
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) {
    info!("Initializing");

    let peripherals: Peripherals = esp_hal::init(esp_hal::Config::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);

    esp_hal_embassy::init(timg0.timer0);

    info!("Initializing compete");

    // Set SPI bus
    let spi_clock = peripherals.GPIO4.degrade();
    let spi_miso = peripherals.GPIO3.degrade();
    let spi_mosi = peripherals.GPIO2.degrade();

    let spi_bus = spi::init(
        peripherals.DMA,
        peripherals.SPI2,
        spi_clock,
        spi_mosi,
        spi_miso,
    );

    let lora_spi_csb = Output::new(peripherals.GPIO1.degrade(), Level::High);

    let lora_spi= SpiDevice::new(spi_bus, lora_spi_csb);

    let lora_rst = Output::new(peripherals.GPIO6.degrade(), Level::High);
    let lora_irq = Input::new(peripherals.GPIO5.degrade(),  Pull::Up);

    spawner
        .spawn(lora::receive(lora_spi, lora_irq, lora_rst))
        .ok();
}
