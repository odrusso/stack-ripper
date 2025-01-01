#![deny(unsafe_code)]
#![no_main]
#![no_std]


use embassy_executor::{task, Spawner};
use embassy_time::Timer;

use esp_hal::{
    clock::{ClockControl, CpuClock},
    dma::Dma,
    embassy,
    i2c::I2C,
    peripherals::Peripherals,
    prelude::*, 
    spi::{master::Spi, SpiMode},
    timer::TimerGroup,
    uart::{config::Config, TxRxPins},
    Uart, IO,
};

use defmt::info;
use esp_backtrace as _;

mod alt;
mod gps;
mod lora;
mod state;

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

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();
    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    info!("Initializing compete");

    // Setup UART for GPS
    // let uart_pins = Some(TxRxPins::new_tx_rx(io.pins.gpio21, io.pins.gpio20));
    // let uart_config = Config::default().baudrate(9200);
    // let uart = Uart::new_with_config(peripherals.UART0, uart_config, uart_pins, &clocks);

    // let (_, rx) = uart.split();

    // Note that this task now owns the UART RX line completely
    // _spawner.spawn(gps::sample(rx)).unwrap();

    // Setup I2C for barometer
    // let bmp_i2c_clock = io.pins.gpio8;
    // let bmp_i2c_data = io.pins.gpio9;
    // let i2c = I2C::new(
    //     peripherals.I2C0,
    //     bmp_i2c_data,
    //     bmp_i2c_clock,
    //     800_u32.kHz(),
    //     &clocks,
    // );

    // Note that this task now owns the I2C bus completely
    // _spawner.spawn(alt::sample(i2c)).ok();

    // Set SPI for LoRa
    let lora_spi_clock = io.pins.gpio4;
    let lora_spi_miso = io.pins.gpio3;
    let lora_spi_mosi = io.pins.gpio2;
    let lora_spi_csb = io.pins.gpio1.into_push_pull_output();

    let lora_rst = io.pins.gpio6.into_push_pull_output();
    let lora_irq = io.pins.gpio5.into_pull_up_input();

    let spi = Spi::new(peripherals.SPI2, 200_u32.kHz(), SpiMode::Mode0, &clocks)
        .with_sck(lora_spi_clock)
        .with_mosi(lora_spi_mosi)
        .with_miso(lora_spi_miso);

    let dma = Dma::new(peripherals.DMA);
    let dma_channel = dma.channel0;

    _spawner
        .spawn(lora::transmit(
            spi,
            lora_irq.into(),
            lora_rst.into(),
            lora_spi_csb.into(),
            dma_channel,
        ))
        .ok();

    // Finally set up the task to print state
    // _spawner.spawn(print_state()).ok();
}
