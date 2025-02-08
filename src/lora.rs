use defmt::{error, info};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::task;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{with_timeout, Delay, Duration, Timer};
use esp_hal::{
    gpio::{AnyPin, Input, Output},
    spi::master::SpiDmaBus,
    Async,
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
    spi: SpiDevice<'static, NoopRawMutex, SpiDmaBus<'static, Async>, Output<'static, AnyPin>>,
    lora_irq: Input<'static, AnyPin>,
    lora_rst: Output<'static, AnyPin>,
) -> ! {
    // We're using an SX1278, but the SX1276 variant seems to work
    let config = sx127x::Config {
        chip: sx127x::Sx1276,
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
            LORA_MAX_PACKET_SIZE_BYTES as u8,
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

        info!("Preparing for RX");
        let prepare_rx_timeout_result = with_timeout(
            Duration::from_secs(10),
            lora.prepare_for_rx(
                RxMode::Continuous,
                &modulation_parameters,
                &rx_packet_parameters,
            ),
        );

        match prepare_rx_timeout_result.await {
            Ok(Ok(_)) => {}
            Ok(Err(_)) => {
                error!("Prepare RX failed");
                continue;
            }
            Err(_) => {
                error!("Prepare RX timed out after 10 seconds");
                continue;
            }
        };

        info!("Waiting up to 60s for LoRA message...");
        let rx_timeout_result = with_timeout(
            Duration::from_secs(60),
            lora.rx(&rx_packet_parameters, &mut rx_buff),
        );

        match rx_timeout_result.await {
            Ok(Ok((received_len, _rx_pkt_status))) => {
                info!("RX successful, with {} bytes", received_len);
                // Deserialize and print
                let out: State = from_bytes(&rx_buff).unwrap();
                info!(
                    "Received state: {:?}, RSSI: {}, SNR: {}",
                    out, _rx_pkt_status.rssi, _rx_pkt_status.snr
                );
            }
            Ok(Err(_)) => {
                error!("RX failed");
                continue;
            }
            Err(_) => {
                error!("RX timed out after 60 seconds");
                continue;
            }
        };
    }
}

#[task]
pub async fn transmit(
    spi: SpiDevice<'static, NoopRawMutex, SpiDmaBus<'static, Async>, Output<'static, AnyPin>>,
    lora_irq: Input<'static, AnyPin>,
    lora_rst: Output<'static, AnyPin>,
) -> ! {
    // We're using an SX1278, but the SX1276 variant seems to work
    let config = sx127x::Config {
        chip: sx127x::Sx1276,
        tcxo_used: false,
        rx_boost: false,
        tx_boost: true,
    };

    let interface_variant =
        GenericSx127xInterfaceVariant::new(lora_rst, lora_irq, None, None).unwrap();

    let sx_device = Sx127x::new(spi, interface_variant, config);

    let mut lora = LoRa::new(sx_device, false, Delay).await.unwrap();

    // Do we need to init??
    lora.init().await.unwrap();

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
        Timer::after_millis(3_000).await;

        // TODO: Can we move setting up this beff to outside the loop?
        let mut buff = [0u8; LORA_MAX_PACKET_SIZE_BYTES];
        let output = to_slice(&*STATE.lock().await, &mut buff).unwrap();

        info!("Transmitting {:?} bytes over LoRA", output.len());
        let prepare_tx_timeout_result = with_timeout(
            Duration::from_millis(100),
            lora.prepare_for_tx(
                &modulation_parameters,
                &mut tx_packet_parameters,
                20,
                &output,
            ),
        );

        match prepare_tx_timeout_result.await {
            Ok(Ok(_)) => {
                info!("Prepare TX succeeded")
            }
            Ok(Err(_)) => {
                error!("Prepare TX failed");
                continue;
            }
            Err(_) => {
                error!("Prepare TX timed out after 10 seconds");
                continue;
            }
        };

        let tx_timeout_result = with_timeout(Duration::from_secs(30), lora.tx());

        match tx_timeout_result.await {
            Ok(Ok(r)) => {
                info!("TX succeeded");
                r
            }
            Ok(Err(_)) => {
                error!("TX failed");
                continue;
            }
            Err(_) => {
                error!("TX timed out after 30 seconds");
                continue;
            }
        };

        info!("LoRA complete");
    }
}

fn create_lora_modulation_parameters<T: RadioKind, U: DelayNs>(
    lora: &mut LoRa<T, U>,
) -> ModulationParams {
    // These settings result in roughly 977 bps
    // The coding rate can be changed to 4/5 to get to 1.6kbps
    // But this is about as reliable as we can get without seriosuly harming
    // bitrate, without having an external TCXO reference clock required
    // for the lower bandwidths to be reliable.
    let params = lora.create_modulation_params(
        SpreadingFactor::_8,
        Bandwidth::_62KHz,
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
