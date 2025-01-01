#![deny(unsafe_code)]
#![no_main]
#![no_std]

use embassy_executor::{task, Spawner};
use embassy_time::Timer;

use esp_hal::{
    peripherals::Peripherals,
    prelude::*,
    timer::timg::TimerGroup,
    uart::{Config, Uart},
};

use defmt::info;
use esp_backtrace as _;

use stack_ripper::{gps, state};

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

    let peripherals: Peripherals = esp_hal::init(esp_hal::Config::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);

    esp_hal_embassy::init(timg0.timer0);

    info!("Initializing compete");

    // Setup UART for GPS
    let tx_pin = peripherals.GPIO7.degrade();
    let rx_pin = peripherals.GPIO8.degrade();

    let uart_config = Config::default().baudrate(9600);
    let uart = Uart::new_with_config(peripherals.UART0, uart_config, tx_pin, rx_pin)
        .unwrap()
        .into_async();

    let (rx, _) = uart.split();

    // Note that this task now owns the UART RX line completely
    // UART is a 1:1 interface, so this is fine
    _spawner.spawn(gps::sample_uart(rx)).unwrap();

    // Setup SPI bus
    // let spi_clock = AnyPin::new(io.pins.gpio20);
    // let spi_miso = AnyPin::new(io.pins.gpio21);
    // let spi_mosi = AnyPin::new(io.pins.gpio1);

    // let spi_bus = spi::init(
    //     peripherals.DMA,
    //     peripherals.SPI2,
    //     &clocks,
    //     spi_clock,
    //     spi_mosi,
    //     spi_miso,
    // );

    // let lora_spi_csb = AnyOutput::new(io.pins.gpio0, Level::High);
    // let lora_spi = SpiDevice::new(spi_bus, lora_spi_csb.into());

    // let lora_rst: AnyOutput<'_> = AnyOutput::new(io.pins.gpio10, Level::High);
    // let lora_irq = AnyInput::new(io.pins.gpio2, Pull::Up);

    // _spawner
    //     .spawn(lora::transmit(lora_spi, lora_irq, lora_rst))
    //     .ok();

    // Finally set up the task to print state
    // _spawner.spawn(print_state()).ok();
}
