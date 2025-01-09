#![deny(unsafe_code)]
#![no_main]
#![no_std]

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;

use esp_hal::{
    gpio::{Input, Level, Output, Pull},
    peripherals::Peripherals,
    prelude::*,
    timer::timg::TimerGroup,
    uart::{Config, Uart},
};

use defmt::info;
use esp_backtrace as _;

use stack_ripper::{gps, lora, spi};

#[main]
async fn main(spawner: Spawner) -> () {
    info!("Initializing");

    let peripherals: Peripherals = esp_hal::init(esp_hal::Config::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);

    esp_hal_embassy::init(timg0.timer0);

    info!("Initializing compete");

    // Setup UART for GPS
    let rx_pin = peripherals.GPIO4.degrade();
    let tx_pin = peripherals.GPIO5.degrade();

    let uart_config = Config::default().baudrate(9600);
    let uart = Uart::new_with_config(peripherals.UART0, uart_config, rx_pin, tx_pin)
        .unwrap()
        .into_async();

    let (rx, _) = uart.split();

    // Note that this task now owns the UART RX line completely
    // UART is a 1:1 interface, so this is fine
    spawner.spawn(gps::sample_uart(rx)).unwrap();

    // Setup SPI bus
    let spi_clock = peripherals.GPIO21.degrade();
    let spi_miso = peripherals.GPIO20.degrade();
    let spi_mosi = peripherals.GPIO10.degrade();

    let spi_bus = spi::init(
        peripherals.DMA,
        peripherals.SPI2,
        spi_clock,
        spi_mosi,
        spi_miso,
    );

    let lora_spi_csb = Output::new(peripherals.GPIO7.degrade(), Level::High);
    let lora_spi = SpiDevice::new(spi_bus, lora_spi_csb);

    let lora_rst = Output::new(peripherals.GPIO3.degrade(), Level::High);
    let lora_irq = Input::new(peripherals.GPIO6.degrade(), Pull::Up);

    spawner
        .spawn(lora::transmit(lora_spi, lora_irq, lora_rst))
        .ok();
}
