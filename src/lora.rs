use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::task;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Delay, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    dma::{Channel0, ChannelCreator0, DmaPriority},
    dma_buffers,
    gpio::{AnyPin, Input, Output, PullUp, PushPull},
    peripherals::SPI2,
    spi::{
        master::{dma::{SpiDma, WithDmaSpi2}, Spi},
        FullDuplexMode,
    },
};
use lora_phy::{
    iv::GenericSx127xInterfaceVariant,
    mod_params::{Bandwidth, CodingRate, ModulationParams, SpreadingFactor},
    mod_traits::RadioKind,
    sx127x::{self, Sx127x},
    DelayNs, LoRa, RxMode,
};
use postcard::{from_bytes, to_slice};

use crate::state::{State, STATE};

const LORA_FREQUENCY_IN_HZ: u32 = 433_000_000;
const LORA_MAX_PACKET_SIZE_BYTES: usize = 255;

#[task]
pub async fn receive(
    spi: Spi<'static, SPI2, FullDuplexMode>,
    lora_irq: AnyPin<Input<PullUp>>,
    lora_rst: AnyPin<Output<PushPull>>,
    lora_spi_csb: AnyPin<Output<PushPull>>,
    dma_channel: ChannelCreator0,
) -> ! {
    let (_, mut tx_descriptors, rx_buffer, mut rx_descriptors) = dma_buffers!(128);

    let spi = spi.with_dma(dma_channel.configure(
        false,
        &mut tx_descriptors,
        &mut rx_descriptors,
        DmaPriority::Priority0,
    ));

    let spi = ExclusiveDevice::new(spi, lora_spi_csb, Delay);

    // We're using an SX1278, but the SX1276 variant seems to work
    let config = sx127x::Config {
        chip: sx127x::Sx127xVariant::Sx1276,
        tcxo_used: false,
        rx_boost: true,
        tx_boost: false,
    };

    let interface_variant =
        GenericSx127xInterfaceVariant::new(lora_rst, lora_irq, None, None).unwrap();

    let mut lora = LoRa::new(Sx127x::new(spi, interface_variant, config), false, Delay)
        .await
        .unwrap();

    let modulation_parameters = create_lora_modulation_parameters(&mut lora);

    let rx_packet_parameters = {
        match lora.create_rx_packet_params(
            16,
            false,
            rx_buffer.len() as u8,
            true,
            false,
            &modulation_parameters,
        ) {
            Ok(pp) => pp,
            Err(err) => {
                panic!("RX Packet Parameters Error: {:?}", err);
            }
        }
    };

    loop {
        // TODO: Can we move this out of the loop?
        let mut rx_buff = [0u8; LORA_MAX_PACKET_SIZE_BYTES];

        match lora
            .prepare_for_rx(
                RxMode::Continuous,
                &modulation_parameters,
                &rx_packet_parameters,
            )
            .await
        {
            Ok(()) => {}
            Err(err) => {
                panic!("Prepare for RX error {:?}", err);
            }
        };

        match lora.rx(&rx_packet_parameters, &mut rx_buff).await {
            Ok((received_len, _rx_pkt_status)) => {
                info!("rx successful with {} bytes", received_len);
                info!(
                    "packet info rssi:{} snr:{}",
                    _rx_pkt_status.rssi, _rx_pkt_status.snr
                );

                // Deserialize and print
                let out: State = from_bytes(&rx_buff).unwrap();
                info!("received state: {:?}", out);
            }
            Err(err) => info!("rx unsuccessful, {:?}", err),
        }
    }
}

#[task]
pub async fn transmit(
    spi: SpiDevice<'static, NoopRawMutex, SpiDma<'static, SPI2, Channel0, FullDuplexMode>, AnyPin<Output<PushPull>>>,
    lora_irq: AnyPin<Input<PullUp>>,
    lora_rst: AnyPin<Output<PushPull>>
) -> ! {
    // We're using an SX1278, but the SX1276 variant seems to work
    let config = sx127x::Config {
        chip: sx127x::Sx127xVariant::Sx1276,
        tcxo_used: false,
        rx_boost: false,
        tx_boost: true,
    };

    let interface_variant =
        GenericSx127xInterfaceVariant::new(lora_rst, lora_irq, None, None).unwrap();

    let sx_device = Sx127x::new(spi, interface_variant, config);

    let mut lora = LoRa::new(sx_device, false, Delay)
        .await
        .unwrap();

    let modulation_parameters = create_lora_modulation_parameters(&mut lora);

    let mut tx_packet_parameters = {
        match lora.create_tx_packet_params(16, false, true, false, &modulation_parameters) {
            Ok(pp) => pp,
            Err(err) => {
                panic!("TX Param Setup: {:?}", err);
            }
        }
    };

    loop {
        // TODO: Can we move setting up this beff to outside the loop?
        let mut buff = [0u8; LORA_MAX_PACKET_SIZE_BYTES];
        let output = to_slice(&*STATE.lock().await, &mut buff).unwrap();

        info!("Transmitting {:?} bytes over LoRA", output.len());        
        lora.prepare_for_tx(
            &modulation_parameters,
            &mut tx_packet_parameters,
            20,
            &output,
        )
        .await
        .unwrap_or(());

        // There's not much we can do if this fails
        lora.tx().await.unwrap_or(());

        info!("LoRA complete");

        // Only transmit once every 10 seconds
        // Maybe this should be longer...?
        Timer::after_millis(10_000).await;
    }
}

fn create_lora_modulation_parameters<T: RadioKind, U: DelayNs>(
    lora: &mut LoRa<T, U>,
) -> ModulationParams {
    let params = lora.create_modulation_params(
        SpreadingFactor::_10,
        Bandwidth::_15KHz,
        CodingRate::_4_8,
        LORA_FREQUENCY_IN_HZ,
    );

    match params {
        Ok(mp) => return mp,
        Err(err) => {
            panic!("Modulation Param Setup: {:?}", err);
        }
    }
}
