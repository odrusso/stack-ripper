use defmt::{error, info};
use embassy_executor::task;
use embedded_io_async::Read;
use esp_hal::{peripherals::UART0, uart::{AnyUart, UartRx}, Async};
use nmea0183::{ParseResult, Parser, Sentence};

use crate::state::STATE;

#[task]
pub async fn sample_uart(mut rx: UartRx<'static, Async, AnyUart>) -> ! {
    // Apparently NMEA sentences are always 79 bytes long, but we'll give this a buffer
    let mut read_buffer: [u8; 1] = [0u8; 1];

    // We only want GGA sentences parsed, which contains the main GPS info we need
    // let mut parser = Parser::new().sentence_only(Sentence::GGA);
    let mut parser: Parser = Parser::new().sentence_only(Sentence::GGA);

    loop {
        info!("waiting for a byte");

        // Read one byte
        // let read_result = Read::read(&mut rx, &mut read_buffer).await;

        // match read_result {
        //     Ok(_) => {
        //         read_result.unwrap();
        //     }
        //     Err(_) => {
        //         // Should we empty the buffer here?
        //         continue;
        //     }
        // }

        info!("Read a byte");

        let parsed_message = parser.parse_from_byte(read_buffer[0]);

        // Not enough info to prase a message yet
        if parsed_message.is_none() {
            continue;
        }

        // Parsed message but be something, match it
        match parsed_message.unwrap() {
            Ok(ParseResult::GGA(Some(gps_result))) => {
                info!("Location result parsed.");
                let mut state = STATE.lock().await;
                state.lt = Some(gps_result.latitude.as_f64() as f32);
                state.ln = Some(gps_result.longitude.as_f64() as f32);
                state.ga = Some(gps_result.altitude.meters);
            }
            Ok(ParseResult::GGA(None)) => {
                info!("Location result recieved, but with empty information");
            }
            Ok(_) => {
                /* Other results parsed. This shouldn't happen because of the filter */
                error!("Some other result recieved from GPS\n");
            }
            Err(e) => {
                error!("NMEA Parse Error: {:?}\n", e);
            }
        }
    }
}
