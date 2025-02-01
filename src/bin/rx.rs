#![deny(unsafe_code)]
#![no_main]
#![no_std]

use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_hal::{
    gpio::{AnyPin, Input, Level, Output, Pin, Pull},
    peripherals::{Peripherals, DMA, SPI2, TIMG0},
    timer::timg::TimerGroup,
};

use stack_ripper::{lora, spi, state};

struct RxPins {
    
    lora_rst: AnyPin,
    lora_irq: AnyPin,

    lora_nss: AnyPin,
    lora_mosi: AnyPin,
    lora_miso: AnyPin,
    lora_clk: AnyPin,

    timg: TIMG0,
    dma: DMA,
    spi: SPI2,
}

fn get_rx_pins_v001(p: Peripherals) -> RxPins {
    RxPins {
        lora_rst: p.GPIO6.degrade(),
        lora_irq: p.GPIO5.degrade(),

        lora_nss: p.GPIO1.degrade(),
        lora_clk: p.GPIO4.degrade(),
        lora_miso: p.GPIO3.degrade(),
        lora_mosi: p.GPIO2.degrade(),

        timg: p.TIMG0,
        dma: p.DMA,
        spi: p.SPI2,
    }
}

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

    let pins = get_rx_pins_v001(peripherals);

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
    let lora_irq = Input::new(pins.lora_irq.degrade(), Pull::Up);

    spawner
        .spawn(lora::receive(lora_spi, lora_irq, lora_rst))
        .ok();
}
