#![deny(unsafe_code)]
#![no_main]
#![no_std]

use embassy_executor::{task, Spawner};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Timer;

use embedded_io_async::Read;
use esp_backtrace as _;
use esp_hal::{
    clock::{ClockControl, CpuClock}, embassy, gpio::{AnyPin, Output, PushPull, NO_PIN}, i2c::I2C, peripherals::{Peripherals, I2C0, SPI2, UART0}, prelude::*, spi::{master::Spi, FullDuplexMode, SpiMode}, timer::TimerGroup, uart::{config::Config, TxRxPins}, Delay, Uart, UartRx, IO
};
use esp_println::println;

use micromath::F32Ext;
use bme280::i2c::AsyncBME280;
use nmea0183::{ParseResult, Parser, Sentence};

use heapless::Vec;
use serde::{Serialize, Deserialize};
use postcard::{from_bytes, to_vec};


// Global state crap
#[derive(Debug, Serialize, Deserialize)]
struct State {
    longitude: f32,
    latitude: f32,
    gps_altitude: f32,
    altimeter_altitude: f32,
}

static STATE: Mutex<CriticalSectionRawMutex, State> = Mutex::new(State {longitude: 0.0, latitude: 0.0, gps_altitude: 0.0, altimeter_altitude: 0.0});

// TODO: This is no longer necessary when a new version of esp-hal drops (greater than 0.16.1)
struct FutureDelay(Delay);
impl embedded_hal_async::delay::DelayNs for FutureDelay {
    async fn delay_ns(&mut self, ns: u32) {
        self.0.delay_nanos(ns);
    }
}

fn get_absolute_altitude_from_pressure(pressure: f32) -> f32 {
    // TODO This isn't giving me the numbers I expect
    const SEA_LEVEL_PRESSURE_HPA: f32 = 101325_f32;
    44_330_f32 * (1_f32 - f32::powf(pressure / SEA_LEVEL_PRESSURE_HPA, 0.1903_f32))
}

#[task]
async fn sample_altitude(mut delay: FutureDelay, i2c: I2C<'static, I2C0>) -> ! {
    let mut alitmeter = AsyncBME280::new_primary(i2c);
    alitmeter.init(&mut delay).await.unwrap();

    loop {
        let current_alt = alitmeter.measure(&mut delay).await.unwrap();
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
                        },                 
                        Ok(_) => { /* Other results parsed. This shouldn't happen because of the filter */ },
                        Err(e) => { println!("NMEA Parse Error: {:?}", e) }
                    }
                }
            }
            Err(e) => println!("RX Error: {:?}", e),
        }
    }
}

#[task]
async fn recieve_lora(spi: Spi<'static, SPI2, FullDuplexMode>, lora_spi_csb: AnyPin<Output<PushPull>>, lora_rst: AnyPin<Output<PushPull>>, delay: Delay) -> ! {

    let lora = sx127x_lora::LoRa::new(
        spi,
        lora_spi_csb,
        lora_rst,
        433,
        delay
    );

    match lora {
        Ok(_) => println!("lora succes"),
        Err(ref x) => println!("error {:?}", x),
    };

    let mut lora = lora.unwrap();

    loop {
        let poll = lora.poll_irq(Some(10_000)); // Timeout after 10 seconds
        match poll {

            Ok(_) =>{
               println!("Recieved packet: ");
               let buffer = lora.read_packet().unwrap(); // Received buffer. NOTE: 255 bytes are always returned

               // Deserialize and print
               let out: State = from_bytes(&buffer).unwrap();
               println!("recieved state: {:?}", out);
            },

            Err(err) => println!("Timeout {:?}", err),
        }

        // Idk how nicely the LoRA module will play with this async stuff here :/
        // Timer::after_millis(100).await;
    }
}

#[task]
async fn transmit_lora(spi: Spi<'static, SPI2, FullDuplexMode>, lora_spi_csb: AnyPin<Output<PushPull>>, lora_rst: AnyPin<Output<PushPull>>, delay: Delay) -> ! {

    let lora = sx127x_lora::LoRa::new(
        spi,
        lora_spi_csb,
        lora_rst,
        433,
        delay
    );

    match lora {
        Ok(_) => println!("lora succes"),
        Err(ref x) => println!("error {:?}", x),
    };

    let mut lora = lora.unwrap();

    loop {
        let output: Vec<u8, 255> = to_vec(&*STATE.lock().await).unwrap();

        let mut buff = [0u8; 255];

        buff[..output.len()].clone_from_slice(&output);
        
        let transmit = lora.transmit_payload(
            buff,
            output.len()
        );

        match transmit {
            Ok(_) => println!("Sent packet"),
            Err(e) => println!("Error: {:?}", e),
        }   

        // Idk how nicely the LoRA module will play with this async stuff here :/
        Timer::after_millis(2_000).await;
    }
}

#[task]
async fn print_state() -> ! {
    loop {
        println!("{:?}", *STATE.lock().await);
        Timer::after_millis(60_000).await;
    }
}

#[main]
async fn main(_spawner: Spawner) -> () {
    println!("Initializing");

    let peripherals = Peripherals::take();

    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();
    let delay = FutureDelay(Delay::new(&clocks));

    let timg0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    embassy::init(&clocks, timg0);

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    // Setup UART for GPS
    let uart_pins = Some(TxRxPins::new_tx_rx(io.pins.gpio21, io.pins.gpio20));
    let uart_config = Config::default().baudrate(9200);
    let uart = Uart::new_with_config(peripherals.UART0, uart_config, uart_pins, &clocks);
    let (_, rx) = uart.split();

    // Note that this task now owns the UART RX line completely
    // _spawner.spawn(sample_gps(rx)).unwrap();    

    // Setup I2C for GPS
    let bmp_i2c_clock = io.pins.gpio8;
    let bmp_i2c_data = io.pins.gpio9;
    let i2c = I2C::new(peripherals.I2C0, bmp_i2c_data, bmp_i2c_clock, 800_u32.kHz(), &clocks);

    // Note that this task now owns the I2C bus completely
    // _spawner.spawn(sample_altitude(FutureDelay(delay.0.clone()), i2c)).ok();

    // Set SPI for LoRa
    let lora_spi_clock = io.pins.gpio0;
    let lora_spi_miso = io.pins.gpio1;
    let lora_spi_mosi = io.pins.gpio2;
    let lora_spi_csb = io.pins.gpio3.into_push_pull_output();
    let lora_rst = io.pins.gpio10.into_push_pull_output();

    let spi = Spi::new(peripherals.SPI2, 800_u32.kHz(), SpiMode::Mode0, &clocks);

    // TODO Can we do NO_PIN here?
    let spi = spi.with_pins(Some(lora_spi_clock), Some(lora_spi_mosi), Some(lora_spi_miso), NO_PIN);

    _spawner.spawn(recieve_lora(spi, lora_spi_csb.into(), lora_rst.into(), delay.0.clone())).ok();

    // Finally setup the task to handle telemetry
    _spawner.spawn(print_state()).ok();
}
