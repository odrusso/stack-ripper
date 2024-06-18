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
    uart::{config::Config, TxRxPins, Uart},
};

use defmt::info;
use esp_backtrace as _;

use stack_ripper::{gps, lora, spi, state};

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

    // Setup UART for GPS
    let uart_pins = Some(TxRxPins::new_tx_rx(io.pins.gpio21, io.pins.gpio20));
    let uart_config = Config::default().baudrate(9200);
    let uart = Uart::new_async_with_config(peripherals.UART0, uart_config, uart_pins, &clocks);

    let (_, rx) = uart.split();

    // Note that this task now owns the UART RX line completely
    // UART is a 1:1 interface, so this is fine
    _spawner.spawn(gps::sample_uart(rx)).unwrap();

    // Setup SPI bus
    let spi_clock = AnyPin::new(io.pins.gpio0);
    let spi_miso = AnyPin::new(io.pins.gpio1);
    let spi_mosi = AnyPin::new(io.pins.gpio2);

    let spi_bus = spi::init(
        peripherals.DMA,
        peripherals.SPI2,
        &clocks,
        spi_clock,
        spi_mosi,
        spi_miso,
    );

    let lora_spi_csb = AnyOutput::new(io.pins.gpio3, Level::Low);
    let lora_spi_device = SpiDevice::new(spi_bus, lora_spi_csb);

    let lora_rst = AnyOutput::new(io.pins.gpio9, Level::High);
    let lora_irq = AnyInput::new(io.pins.gpio10, Pull::Up);

    _spawner
        .spawn(lora::transmit(lora_spi_device, lora_irq, lora_rst))
        .ok();

    // Finally set up the task to print state
    _spawner.spawn(print_state()).ok();
}
