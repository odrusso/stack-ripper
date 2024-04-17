use defmt::warn;
use embassy_executor::task;
use embedded_io_async::Read;
use esp_hal::{peripherals::UART0, UartRx};
use nmea0183::{ParseResult, Parser, Sentence};

use crate::state::STATE;

#[task]
pub async fn sample(mut rx: UartRx<'static, UART0>) -> ! {
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
