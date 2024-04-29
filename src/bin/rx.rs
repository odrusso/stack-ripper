#![deny(unsafe_code)]
#![no_main]
#![no_std]

use embassy_executor::{task, Spawner};
use embassy_time::Timer;

use esp_hal::{
    clock::{ClockControl, CpuClock},
    dma::Dma,
    embassy,
    peripherals::Peripherals,
    prelude::*, 
    spi::{master::Spi, SpiMode},
    timer::TimerGroup,
    IO,
};

use defmt::info;
use esp_backtrace as _;

use stack_ripper::{state, lora};

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

    // Set SPI for LoRa
    let lora_spi_clock = io.pins.gpio3;
    let lora_spi_miso = io.pins.gpio2;
    let lora_spi_mosi = io.pins.gpio1;
    let lora_spi_csb = io.pins.gpio0.into_push_pull_output();

    let lora_rst = io.pins.gpio10.into_push_pull_output();
    let lora_irq = io.pins.gpio4.into_pull_up_input();

    let spi = Spi::new(peripherals.SPI2, 200_u32.kHz(), SpiMode::Mode0, &clocks)
        .with_sck(lora_spi_clock)
        .with_mosi(lora_spi_mosi)
        .with_miso(lora_spi_miso);

    let dma = Dma::new(peripherals.DMA);
    let dma_channel = dma.channel0;

    _spawner
        .spawn(lora::receive(
            spi,
            lora_irq.into(),
            lora_rst.into(),
            lora_spi_csb.into(),
            dma_channel,
        ))
        .ok();
}
