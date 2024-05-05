#![deny(unsafe_code)]
#![no_main]
#![no_std]


use embassy_embedded_hal::shared_bus::asynch::{i2c::I2cDevice, spi::SpiDevice};
use embassy_executor::{task, Spawner};
use embassy_time::Timer;

use esp_hal::{
    clock::{ClockControl, CpuClock},
    embassy,
    peripherals::Peripherals,
    prelude::*, 
    timer::TimerGroup,
    uart::{config::Config, TxRxPins},
    Uart, IO,
};

use defmt::info;
use esp_backtrace as _;

use stack_ripper::{alt, gps, i2c, lora, spi, state};


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
    let uart_pins = Some(TxRxPins::new_tx_rx(io.pins.gpio21, io.pins.gpio20));
    let uart_config = Config::default().baudrate(9200);
    let uart = Uart::new_with_config(peripherals.UART0, uart_config, uart_pins, &clocks);

    let (_, rx) = uart.split();

    // Note that this task now owns the UART RX line completely
    _spawner.spawn(gps::sample_uart(rx)).unwrap();

    // Setup I2C bus
    let i2c_clock = io.pins.gpio8;
    let i2c_data = io.pins.gpio9;
    let i2c_bus = i2c::init(peripherals.I2C0, &clocks, i2c_clock.degrade(), i2c_data.degrade());

    let i2c_alt = I2cDevice::new(i2c_bus);
    _spawner.spawn(alt::sample(i2c_alt)).ok();

    // Setup SPI bus
    let spi_clock = io.pins.gpio0;
    let spi_miso = io.pins.gpio1;
    let spi_mosi = io.pins.gpio2;

    let spi_bus = spi::init(
        peripherals.DMA, 
        peripherals.SPI2,
        &clocks, 
        spi_clock.degrade(), 
        spi_mosi.degrade(), 
        spi_miso.degrade()
    );

    let lora_spi_csb = io.pins.gpio3.into_push_pull_output();
    let lora_spi_device = SpiDevice::new(spi_bus, lora_spi_csb.into());

    let lora_rst = io.pins.gpio10.into_push_pull_output();
    let lora_irq = io.pins.gpio4.into_pull_up_input();

    _spawner
        .spawn(lora::transmit(
            lora_spi_device,
            lora_irq.into(),
            lora_rst.into(),
        ))
        .ok();

    // Finally set up the task to print state
    _spawner.spawn(print_state()).ok();
}
