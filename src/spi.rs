use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use fugit::RateExtU32;
use static_cell::StaticCell;

use esp_hal::{
    clock::Clocks,
    dma::{Channel0, Dma, DmaDescriptor, DmaPriority},
    dma_descriptors,
    gpio::{AnyPin, InputOutputAnalogPinType, Unknown},
    peripherals::{DMA, SPI2},
    spi::{
        master::{dma::SpiDma, prelude::*, Spi},
        FullDuplexMode, SpiMode,
    },
};

static SPI_BUS: StaticCell<Mutex<NoopRawMutex, SpiDma<'static, SPI2, Channel0, FullDuplexMode>>> =
    StaticCell::new();
static DMA_DESCRIPTORS: StaticCell<([DmaDescriptor; 8], [DmaDescriptor; 8])> = StaticCell::new();

pub fn init(
    dma: DMA,
    spi: SPI2,
    clocks: &Clocks,
    sck: AnyPin<Unknown, InputOutputAnalogPinType>,
    mosi: AnyPin<Unknown, InputOutputAnalogPinType>,
    miso: AnyPin<Unknown, InputOutputAnalogPinType>,
) -> &'static mut Mutex<NoopRawMutex, SpiDma<'static, SPI2, Channel0, FullDuplexMode>> {
    let dma = Dma::new(dma);
    let dma_channel = dma.channel0;

    let dma_descriptors = DMA_DESCRIPTORS.init(dma_descriptors!(32000));

    // Max bitrate of the SX1278 is 300kbps vs. 2.2mbps for the NEO-M8. We have to pick the lower of the two.
    let spi = Spi::new(spi, 300_u32.kHz(), SpiMode::Mode0, &clocks)
        .with_sck(sck)
        .with_mosi(mosi)
        .with_miso(miso)
        .with_dma(dma_channel.configure(
            false,
            &mut dma_descriptors.0,
            &mut dma_descriptors.1,
            DmaPriority::Priority0,
        ));

    SPI_BUS.init(Mutex::new(spi))
}
