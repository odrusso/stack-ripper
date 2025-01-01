use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use fugit::RateExtU32;
use static_cell::StaticCell;

use esp_hal::{
    dma::{Dma, DmaPriority, DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::AnyPin,
    peripherals::{DMA, SPI2},
    spi::{
        master::{Config, Spi, SpiDmaBus},
        SpiMode,
    },
    Async,
};

static SPI_BUS: StaticCell<Mutex<NoopRawMutex, SpiDmaBus<'static, Async>>> = StaticCell::new();

pub fn init(
    dma: DMA,
    spi: SPI2,
    sck: AnyPin,
    mosi: AnyPin,
    miso: AnyPin,
) -> &'static mut Mutex<NoopRawMutex, SpiDmaBus<'static, Async>> {
    let dma = Dma::new(dma);
    let dma_channel = dma.channel0;

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(32000);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    let spi_config = Config {
        frequency: 200.kHz(),
        mode: SpiMode::Mode0,
        ..Config::default()
    };

    // Max bitrate of the SX1278 is 300kbps vs. 2.2mbps for the NEO-M8. We have to pick the lower of the two.
    let spi = Spi::new_with_config(spi, spi_config)
        .with_sck(sck)
        .with_mosi(mosi)
        .with_miso(miso)
        .with_dma(dma_channel.configure(false, DmaPriority::Priority0))
        .with_buffers(dma_rx_buf, dma_tx_buf)
        .into_async();

    SPI_BUS.init(Mutex::new(spi))
}
