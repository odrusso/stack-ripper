use defmt::info;
use embassy_executor::task;
use embedded_io_async::Read;
use esp_hal::{peripherals::UART0, UartRx};
use nmea0183::{ParseResult, Parser, Sentence};

use crate::STATE;

#[task]
pub async fn sample(mut rx: UartRx<'static, UART0>) -> ! {
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
