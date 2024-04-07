#![deny(unsafe_code)]
#![no_main]
#![no_std]

use embassy_executor::{task, Spawner};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::{Delay, Timer};

use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_io_async::Read;

use esp_hal::{
    clock::{ClockControl, CpuClock},
    dma::{ChannelCreator0, Dma, DmaPriority},
    dma_buffers, embassy,
    gpio::{AnyPin, Input, Output, PullUp, PushPull},
    i2c::I2C,
    peripherals::{Peripherals, I2C0, SPI2, UART0},
    prelude::*,
    spi::{
        master::{prelude::*, Spi},
        FullDuplexMode, SpiMode,
    },
    timer::TimerGroup,
    uart::{config::Config, TxRxPins},
    Uart, UartRx, IO,
};

use defmt::{info, Format};
use esp_backtrace as _;

use bme280::i2c::AsyncBME280;
use esp_hal::dma::Channel0;
use esp_hal::spi::master::dma::SpiDma;
use heapless::Vec;
use lora_phy::mod_params::{ModulationParams, RadioError};
use lora_phy::{
    iv::GenericSx127xInterfaceVariant,
    mod_params::{Bandwidth, CodingRate, SpreadingFactor},
    sx127x::{self, Sx127x},
    LoRa, RxMode,
};
use micromath::F32Ext;
use nmea0183::{ParseResult, Parser, Sentence};

use postcard::{from_bytes, to_vec};
use serde::{Deserialize, Serialize};

static LORA_FREQUENCY_IN_HZ: u32 = 433_000_000;

// Global state crap
#[derive(Debug, Format, Serialize, Deserialize)]
struct State {
    longitude: f32,
    latitude: f32,
    gps_altitude: f32,
    altimeter_altitude: f32,
}

static STATE: Mutex<CriticalSectionRawMutex, State> = Mutex::new(State {
    longitude: 0.0,
    latitude: 0.0,
    gps_altitude: 0.0,
    altimeter_altitude: 0.0,
});

fn get_absolute_altitude_from_pressure(pressure: f32) -> f32 {
    // TODO This isn't giving me the numbers I expect
    const SEA_LEVEL_PRESSURE_HPA: f32 = 101325_f32;
    44_330_f32 * (1_f32 - f32::powf(pressure / SEA_LEVEL_PRESSURE_HPA, 0.1903_f32))
}

#[task]
async fn sample_altitude(i2c: I2C<'static, I2C0>) -> ! {
    let mut alitmeter = AsyncBME280::new_primary(i2c);
    alitmeter.init(&mut Delay).await.unwrap();

    loop {
        let current_alt = alitmeter.measure(&mut Delay).await.unwrap();
        {
            let mut state = STATE.lock().await;
            state.altimeter_altitude = get_absolute_altitude_from_pressure(current_alt.pressure);
        }

        Timer::after_millis(1_000).await;
    }
}

#[task]
async fn sample_gps(mut rx: UartRx<'static, UART0>) -> ! {
    // Apparently NMEA sentences are always 79 bytes long
    const NMEA_BUFFER_SIZE: usize = 79;
    let mut read_buffer: [u8; NMEA_BUFFER_SIZE] = [0u8; NMEA_BUFFER_SIZE];

    // We only GGA sentences parsed, which contains the main GPS info we need
    let mut parser = Parser::new().sentence_only(Sentence::GGA);

    loop {
        // Read the exact amount of words for a NEMA sentence
        let recieved_bytes = Read::read_exact(&mut rx, &mut read_buffer).await;
        match recieved_bytes {
            Ok(_) => {
                for result in parser.parse_from_bytes(&read_buffer) {
                    match result {
                        Ok(ParseResult::GGA(Some(result))) => {
                            let mut state = STATE.lock().await;
                            state.latitude = result.latitude.as_f64() as f32;
                            state.longitude = result.longitude.as_f64() as f32;
                            state.gps_altitude = result.altitude.meters;
                        }
                        Ok(_) => {
                            /* Other results parsed. This shouldn't happen because of the filter */
                        }
                        Err(e) => {
                            info!("NMEA Parse Error: {:?}", e)
                        }
                    }
                }
            }
            Err(e) => info!("RX Error: {:?}", e),
        }
    }
}

#[task]
async fn receive_lora(
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

    let modulation_parameters = {
        match create_lora_modulation_parameters(&mut lora) {
            Ok(mp) => mp,
            Err(err) => {
                panic!("Modulation Parameter Error: {:?}", err);
            }
        }
    };

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
        let mut rx_buff = [0u8; 255];

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
async fn transmit_lora(
    spi: Spi<'static, SPI2, FullDuplexMode>,
    lora_irq: AnyPin<Input<PullUp>>,
    lora_rst: AnyPin<Output<PushPull>>,
    lora_spi_csb: AnyPin<Output<PushPull>>,
    dma_channel: ChannelCreator0,
) -> ! {
    let (_, mut tx_descriptors, _, mut rx_descriptors) = dma_buffers!(128);

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
        rx_boost: false,
        tx_boost: true,
    };

    let interface_variant =
        GenericSx127xInterfaceVariant::new(lora_rst, lora_irq, None, None).unwrap();

    let mut lora = LoRa::new(Sx127x::new(spi, interface_variant, config), false, Delay)
        .await
        .unwrap();

    let modulation_parameters = {
        match create_lora_modulation_parameters(&mut lora) {
            Ok(mp) => mp,
            Err(err) => {
                panic!("Modulation Param Setup: {:?}", err);
            }
        }
    };

    let mut tx_packet_parameters = {
        match lora.create_tx_packet_params(16, false, true, false, &modulation_parameters) {
            Ok(pp) => pp,
            Err(err) => {
                panic!("TX Param Setup: {:?}", err);
            }
        }
    };

    loop {
        info!("TX START");

        let output: Vec<u8, 128> = to_vec(&*STATE.lock().await).unwrap();
        let mut buff = [0u8; 255];
        buff[..output.len()].clone_from_slice(&output);

        match lora
            .prepare_for_tx(
                &modulation_parameters,
                &mut tx_packet_parameters,
                20,
                &buff[0..output.len()],
            )
            .await
        {
            Ok(()) => {}
            Err(err) => {
                panic!("Prepare TX error: {:?}", err);
            }
        };

        match lora.tx().await {
            Ok(()) => {
                info!("TX DONE");
            }
            Err(err) => {
                panic!("Actual TX error: {:?}", err);
            }
        };

        // info!("Waiting 5 seconds before transmitting");
        // Timer::after_millis(5_000).await;
    }
}

fn create_lora_modulation_parameters(
    lora: &mut LoRa<
        Sx127x<
            ExclusiveDevice<
                SpiDma<SPI2, Channel0, FullDuplexMode>,
                AnyPin<Output<PushPull>>,
                Delay,
            >,
            GenericSx127xInterfaceVariant<AnyPin<Output<PushPull>>, AnyPin<Input<PullUp>>>,
        >,
        Delay,
    >,
) -> Result<ModulationParams, RadioError> {
    lora.create_modulation_params(
        SpreadingFactor::_10,
        Bandwidth::_15KHz,
        CodingRate::_4_8,
        LORA_FREQUENCY_IN_HZ,
    )
}

#[task]
async fn print_state() -> ! {
    loop {
        info!("{:?}", *STATE.lock().await);
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

    // Setup UART for GPS
    let uart_pins = Some(TxRxPins::new_tx_rx(io.pins.gpio21, io.pins.gpio20));
    let uart_config = Config::default().baudrate(9200);
    let uart = Uart::new_with_config(peripherals.UART0, uart_config, uart_pins, &clocks);
    let (_, rx) = uart.split();

    // Note that this task now owns the UART RX line completely
    _spawner.spawn(sample_gps(rx)).unwrap();

    // Setup I2C for barometer
    let bmp_i2c_clock = io.pins.gpio8;
    let bmp_i2c_data = io.pins.gpio9;
    let i2c = I2C::new(peripherals.I2C0, bmp_i2c_data, bmp_i2c_clock, 800_u32.kHz(), &clocks);

    // Note that this task now owns the I2C bus completely
    _spawner.spawn(sample_altitude(i2c)).ok();

    // Set SPI for LoRa
    let lora_spi_clock = io.pins.gpio0;
    let lora_spi_miso = io.pins.gpio1;
    let lora_spi_mosi = io.pins.gpio2;
    let lora_spi_csb = io.pins.gpio3.into_push_pull_output();

    let lora_rst = io.pins.gpio10.into_push_pull_output();
    let lora_irq = io.pins.gpio4.into_pull_up_input();

    let spi = Spi::new(peripherals.SPI2, 200_u32.kHz(), SpiMode::Mode0, &clocks)
        .with_sck(lora_spi_clock)
        .with_mosi(lora_spi_mosi)
        .with_miso(lora_spi_miso);

    let dma = Dma::new(peripherals.DMA);
    let dma_channel = dma.channel0;

    _spawner
        .spawn(transmit_lora(
            spi,
            lora_irq.into(),
            lora_rst.into(),
            lora_spi_csb.into(),
            dma_channel,
        ))
        .ok();

    // Finally set up the task to print state
    // _spawner.spawn(print_state()).ok();
}
