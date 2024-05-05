#![deny(unsafe_code)]
#![no_main]
#![no_std]

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::{task, Spawner};
use embassy_time::Timer;

use esp_hal::{
    clock::{ClockControl, CpuClock},
    embassy,
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    IO,
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

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();
    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    info!("Initializing compete");

    // Setup shared SPI bus
    let spi_clck_pin = io.pins.gpio0;
    let spi_miso_pin = io.pins.gpio1;
    let spi_mosi_pin = io.pins.gpio2;

    let spi_bus = spi::init(
        peripherals.DMA,
        peripherals.SPI2,
        &clocks,
        spi_clck_pin.degrade(),
        spi_mosi_pin.degrade(),
        spi_miso_pin.degrade(),
    );

    // Setup GPS task
    let gps_csb_pin = io.pins.gpio21.into_push_pull_output();
    let gps_spi_device = SpiDevice::new(spi_bus, gps_csb_pin.into());

    _spawner.spawn(gps::sample_spi(gps_spi_device)).unwrap();

    // Setup LoRA Task
    let lora_rst_pin = io.pins.gpio10.into_push_pull_output();
    let lora_irq_pin = io.pins.gpio4.into_pull_up_input();
    let lora_csb_pin = io.pins.gpio3.into_push_pull_output();

    let lora_spi_device = SpiDevice::new(spi_bus, lora_csb_pin.into());

    _spawner
        .spawn(lora::transmit(
            lora_spi_device,
            lora_irq_pin.into(),
            lora_rst_pin.into(),
        ))
        .ok();

    // Finally set up the task to print state
    _spawner.spawn(print_state()).ok();
}
