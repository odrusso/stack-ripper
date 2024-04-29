use defmt::warn;
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::task;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Timer;
use embedded_io_async::Read;
use esp_hal::{dma::Channel0, gpio::{AnyPin, Output, PushPull}, peripherals::{SPI2, UART0}, spi::{master::dma::SpiDma, FullDuplexMode}, UartRx};
use nmea0183::{ParseResult, Parser, Sentence};

use crate::state::STATE;

// Apparently NMEA sentences are always 79 bytes long
const NMEA_BUFFER_SIZE: usize = 79;

#[task]
pub async fn sample_uart(mut rx: UartRx<'static, UART0>) -> ! {
    // Apparently NMEA sentences are always 79 bytes long
    const NMEA_BUFFER_SIZE: usize = 79;
    let mut read_buffer: [u8; NMEA_BUFFER_SIZE] = [0u8; NMEA_BUFFER_SIZE];

    // We only GGA sentences parsed, which contains the main GPS info we need
    let mut parser = Parser::new().sentence_only(Sentence::GGA);

    loop {
        // Read the exact amount of words for a NEMA sentence
        Read::read_exact(&mut rx, &mut read_buffer).await.unwrap();

        for result in parser.parse_from_bytes(&read_buffer) {
            match result {
                Ok(ParseResult::GGA(Some(result))) => {
                    let mut state = STATE.lock().await;
                    state.lt = Some(result.latitude.as_f64() as f32);
                    state.ln = Some(result.longitude.as_f64() as f32);
                    state.ga = Some(result.altitude.meters);
                }
                Ok(_) => {
                    /* Other results parsed. This shouldn't happen because of the filter */
                    // info!("Some other result recieved from GPS")
                }
                Err(e) => {
                    warn!("NMEA Parse Error: {:?}", e);

                    // I assume we've missed a byte somehow?
                    // Keep reading bytes, 1 at a time, until we hit CRLF

                    let mut eol_buff: [u8; 1] = [0u8; 1];
                    while eol_buff[0] != b'\n' {
                        Read::read_exact(&mut rx, &mut eol_buff).await.unwrap();
                    }
                    
                    // The very last byte we consumed was a LF
                    // Presumably - the byte before that was a CR
                    // Now whatever is next should be the start of another NMEA message
                }
            }
        }
    }
}


#[task]
pub async fn sample_spi(mut spi: SpiDevice<'static, NoopRawMutex, SpiDma<'static, SPI2, Channel0, FullDuplexMode>, AnyPin<Output<PushPull>>>) -> ! {
    let mut read_buffer: [u8; NMEA_BUFFER_SIZE] = [0u8; NMEA_BUFFER_SIZE];

    // Apparently - the way to read from the NEO-M8 is to concurrently write 1s to the MOSI while reading from MISO
    // Unclear if this is what the module expects, maybe this should be all zeros?
    let write_buffer = [0xFF; NMEA_BUFFER_SIZE];

    // We only GGA sentences parsed, which contains the main GPS info we need
    let mut parser = Parser::new().sentence_only(Sentence::GGA);

    loop {
        // Read the exact amount of words for a NEMA sentence
        embedded_hal_async::spi::SpiDevice::transfer(&mut spi, &mut read_buffer, &write_buffer).await.unwrap();

        // If the read buffer is all 0xFF, (1s) then we should stop polling and wait for a bit.
        if read_array_done(&read_buffer) {
            Timer::after_millis(1_000).await;
        }

        for result in parser.parse_from_bytes(&read_buffer) {
            match result {
                Ok(ParseResult::GGA(Some(result))) => {
                    let mut state = STATE.lock().await;
                    state.lt = Some(result.latitude.as_f64() as f32);
                    state.ln = Some(result.longitude.as_f64() as f32);
                    state.ga = Some(result.altitude.meters);
                }
                Ok(_) => {
                    /* Other results parsed. This shouldn't happen because of the filter */
                    // info!("Some other result recieved from GPS")
                }
                Err(e) => {
                    warn!("NMEA Parse Error: {:?}", e);
                }
            }
        }
    }
}

fn read_array_done(read: &[u8; NMEA_BUFFER_SIZE]) -> bool {
    read.iter().all(|a| *a == 0xFF)
}
