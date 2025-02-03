#![deny(unsafe_code)]

use esp_hal::{
    gpio::{AnyPin, Pin},
    peripherals::{Peripherals, DMA, SPI2, TIMG0, UART0},
};

pub struct TxPins {
    pub uart_rx: AnyPin,
    pub uart_tx: AnyPin,

    pub lora_rst: AnyPin,
    pub lora_irq: AnyPin,

    pub lora_nss: AnyPin,
    pub lora_mosi: AnyPin,
    pub lora_miso: AnyPin,
    pub lora_clk: AnyPin,

    pub timg: TIMG0,
    pub uart: UART0,
    pub dma: DMA,
    pub spi: SPI2,
}

pub struct RxPins {
    pub lora_rst: AnyPin,
    pub lora_irq: AnyPin,

    pub lora_nss: AnyPin,
    pub lora_mosi: AnyPin,
    pub lora_miso: AnyPin,
    pub lora_clk: AnyPin,

    pub timg: TIMG0,
    pub dma: DMA,
    pub spi: SPI2,
}

pub fn get_tx_pins_v003(p: Peripherals) -> TxPins {
    TxPins {
        uart_rx: p.GPIO4.degrade(),
        uart_tx: p.GPIO5.degrade(),

        lora_rst: p.GPIO6.degrade(), // Yep
        lora_irq: p.GPIO7.degrade(), // Yep

        lora_nss: p.GPIO8.degrade(),   // Yep
        lora_clk: p.GPIO21.degrade(),  // Yep
        lora_miso: p.GPIO20.degrade(), // Yep
        lora_mosi: p.GPIO10.degrade(), // Yep

        timg: p.TIMG0,
        uart: p.UART0,
        dma: p.DMA,
        spi: p.SPI2,
    }
}

pub fn get_tx_pins_v004_bread(p: Peripherals) -> TxPins {
    TxPins {
        uart_rx: p.GPIO4.degrade(),
        uart_tx: p.GPIO5.degrade(),

        lora_rst: p.GPIO1.degrade(),
        lora_irq: p.GPIO8.degrade(),

        lora_nss: p.GPIO9.degrade(),
        lora_clk: p.GPIO21.degrade(),
        lora_miso: p.GPIO20.degrade(),
        lora_mosi: p.GPIO10.degrade(),

        timg: p.TIMG0,
        uart: p.UART0,
        dma: p.DMA,
        spi: p.SPI2,
    }
}

pub fn get_rx_pins_og(p: Peripherals) -> RxPins {
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

pub fn get_rx_pins_v001(p: Peripherals) -> RxPins {
    RxPins {
        lora_rst: p.GPIO6.degrade(), // Yep, 6 => RST
        lora_irq: p.GPIO5.degrade(), // Yep, 5 => DIO0

        lora_nss: p.GPIO1.degrade(),  // Yep, 1 => NSS
        lora_clk: p.GPIO4.degrade(),  // Yep, 4 => CLK
        lora_miso: p.GPIO3.degrade(), // Yes, 3 => MISO
        lora_mosi: p.GPIO2.degrade(), // Yes, 2 => MOSI

        timg: p.TIMG0,
        dma: p.DMA,
        spi: p.SPI2,
    }
}
