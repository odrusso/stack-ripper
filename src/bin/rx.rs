#![deny(unsafe_code)]
#![no_main]
#![no_std]

use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::{
    gpio::{Input, Level, Output, Pull},
    peripherals::Peripherals,
    timer::timg::TimerGroup,
};

use stack_ripper::{lora, pins, spi, state};

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

    // let pins = pins::get_rx_pins_v001(peripherals);
    let pins = pins::get_tx_pins_v004_bread(peripherals);

    let timg0 = TimerGroup::new(pins.timg);

    esp_hal_embassy::init(timg0.timer0);

    info!("Initializing compete");

    // Setup SPI bus
    let spi_bus = spi::init(
        pins.dma,
        pins.spi,
        pins.lora_clk,
        pins.lora_mosi,
        pins.lora_miso,
    );

    let lora_spi_csb = Output::new(pins.lora_nss, Level::High);

    let lora_spi = SpiDevice::new(spi_bus, lora_spi_csb);

    let lora_rst = Output::new(pins.lora_rst, Level::High);
    let lora_irq = Input::new(pins.lora_irq, Pull::None);

    spawner
        .spawn(lora::receive(lora_spi, lora_irq, lora_rst))
        .ok();
}
