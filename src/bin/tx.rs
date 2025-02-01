#![deny(unsafe_code)]
#![no_main]
#![no_std]

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;

use esp_hal::{
    gpio::{AnyPin, Input, Level, Output, Pull},
    peripherals::{Peripherals, DMA, SPI2, TIMG0, UART0},
    prelude::*,
    timer::timg::TimerGroup,
    uart::{Config, Uart},
};

use defmt::info;
use esp_backtrace as _;

use stack_ripper::{gps, lora, spi};

struct TxPins {
    uart_rx: AnyPin,
    uart_tx: AnyPin,
    
    lora_rst: AnyPin,
    lora_irq: AnyPin,

    lora_nss: AnyPin,
    lora_mosi: AnyPin,
    lora_miso: AnyPin,
    lora_clk: AnyPin,

    timg: TIMG0,
    uart: UART0,
    dma: DMA,
    spi: SPI2,
}

fn get_tx_pins_v003(p: Peripherals) -> TxPins {
    TxPins {
        uart_rx: p.GPIO4.degrade(),
        uart_tx: p.GPIO5.degrade(),

        lora_rst: p.GPIO6.degrade(),
        lora_irq: p.GPIO7.degrade(),

        lora_nss: p.GPIO8.degrade(),
        lora_clk: p.GPIO21.degrade(),
        lora_miso: p.GPIO20.degrade(),
        lora_mosi: p.GPIO10.degrade(),

        timg: p.TIMG0,
        uart: p.UART0,
        dma: p.DMA,
        spi: p.SPI2,
    }
}

#[main]
async fn main(_spawner: Spawner) -> () {
    info!("Initializing");

    let peripherals: Peripherals = esp_hal::init(esp_hal::Config::default());

    let pins = get_tx_pins_v003(peripherals);

    let timg0 = TimerGroup::new(pins.timg);

    esp_hal_embassy::init(timg0.timer0);

    info!("Initializing compete");

    // Setup UART for GPS
    let uart_config = Config::default().baudrate(9600);
    let uart = Uart::new_with_config(pins.uart, uart_config, pins.uart_rx, pins.uart_tx)
        .unwrap()
        .into_async();

    let (rx, _) = uart.split();

    // Note that this task now owns the UART RX line completely
    // UART is a 1:1 interface, so this is fine
    _spawner.spawn(gps::sample_uart(rx)).unwrap();

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
    let lora_irq = Input::new(pins.lora_irq, Pull::Up);

    _spawner
        .spawn(lora::transmit(lora_spi, lora_irq, lora_rst))
        .ok();
}
