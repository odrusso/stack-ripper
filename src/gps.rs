use defmt::{error, info};
use embassy_executor::task;
use embedded_io_async::Read;
use esp_hal::{uart::{AnyUart, UartRx}, Async};
use nmea0183::{ParseResult, Parser, Sentence};

use crate::state::STATE;

#[task]
pub async fn sample_uart(mut rx: UartRx<'static, Async, AnyUart>) -> ! {
    // Apparently NMEA sentences are always 79 bytes long, but we'll give this a buffer
    let mut read_buffer: [u8; 1] = [0u8; 1];

    // We only want GGA/GLL sentences parsed, which contain the main GPS info we need
    let mut parser: Parser = Parser::new().sentence_filter(Sentence::GGA | Sentence::GLL);

    loop {
        // Read one byte
        let read_result = Read::read(&mut rx, &mut read_buffer).await;

        // info!("Results: {}", u8s_to_str(&read_buffer));

        match read_result {
            Ok(_) => {
                // I think we don't need to do this?
                read_result.unwrap();
            }
            Err(_) => {
                // Should we empty the buffer here?
                error!("read error");
                continue;
            }
        }

        let message = parser.parse_from_byte(read_buffer[0]);

        if message.is_none()
        {
            continue;
        }

        match message.unwrap() {
            Ok(ParseResult::GGA(Some(gps_result))) => {
                info!("GGA Location result parsed.");
                let mut state = STATE.lock().await;
                state.lt = Some(gps_result.latitude.as_f64() as f32);
                state.ln = Some(gps_result.longitude.as_f64() as f32);
                state.ga = Some(gps_result.altitude.meters);
            }
            Ok(ParseResult::GLL(Some(gps_result))) => {
                info!("GLL location result parsed.");
                let mut state = STATE.lock().await;
                state.lt = Some(gps_result.latitude.as_f64() as f32);
                state.ln = Some(gps_result.longitude.as_f64() as f32);
            }
            Ok(ParseResult::GGA(None)) => {
                // info!("Location result recieved, but with empty information");
            }
            Ok(_) => {
                // error!("Some other result recieved from GPS\n");
            }
            Err(e) => {
                // error!("NMEA Parse Error: {:?}\n", e);
            }
        }
    }
}