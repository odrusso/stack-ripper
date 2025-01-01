use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use fugit::RateExtU32;
use static_cell::StaticCell;

use esp_hal::{
    clock::Clocks,
    dma::{Channel0, Dma, DmaDescriptor, DmaPriority},
    dma_descriptors,
    gpio::any_pin::AnyPin,
    peripherals::{DMA, SPI2},
    spi::{
        master::{dma::SpiDma, prelude::*, Spi},
        FullDuplexMode, SpiMode,
    },
    Async,
};

static SPI_BUS: StaticCell<
    Mutex<NoopRawMutex, SpiDma<'static, SPI2, Channel0, FullDuplexMode, Async>>,
> = StaticCell::new();

static DMA_DESCRIPTORS: StaticCell<([DmaDescriptor; 8], [DmaDescriptor; 8])> = StaticCell::new();

pub fn init(
    dma: DMA,
    spi: SPI2,
    clocks: &Clocks,
    sck: AnyPin<'static>,
    mosi: AnyPin<'static>,
    miso: AnyPin<'static>,
) -> &'static mut Mutex<NoopRawMutex, SpiDma<'static, SPI2, Channel0, FullDuplexMode, Async>> {
    let dma = Dma::new(dma);
    let dma_channel = dma.channel0;

    let dma_descriptors = DMA_DESCRIPTORS.init(dma_descriptors!(32000));

    // Max bitrate of the SX1278 is 300kbps vs. 2.2mbps for the NEO-M8. We have to pick the lower of the two.
    let spi = Spi::new(spi, 22.kHz(), SpiMode::Mode0, &clocks)
        .with_sck(sck)
        .with_mosi(mosi)
        .with_miso(miso)
        .with_dma(dma_channel.configure_for_async(
            false,
            &mut dma_descriptors.0,
            &mut dma_descriptors.1,
            DmaPriority::Priority0,
        ));

    SPI_BUS.init(Mutex::new(spi))
}
