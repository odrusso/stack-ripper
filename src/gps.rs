use defmt::{error, info};
use embassy_executor::task;
use embedded_io_async::Read;
use esp_hal::{uart::{AnyUart, UartRx}, Async};
use nmea0183::{datetime::Time, ParseResult, Parser, Sentence};

use crate::state::STATE;

fn get_time(time: Time) -> i32 {
    let h = (time.hours as i32) * 10000;
    let m = (time.minutes as i32) * 100;
    let s = time.seconds as i32;
    h + m + s
}

#[task]
pub async fn sample_uart(mut rx: UartRx<'static, Async, AnyUart>) -> ! {
    // Apparently NMEA sentences are always 79 bytes long, but that doesn't seem to be true
    // We'll just go 1 byte at a time, and let the parser deal with it. 
    let mut read_buffer: [u8; 1] = [0u8; 1];

    // We only want GGA/GLL sentences parsed, which contain the main GPS info we need
    let mut parser: Parser = Parser::new().sentence_filter(Sentence::GGA | Sentence::GLL);

    loop {
        // Read one byte
        let read_result = Read::read(&mut rx, &mut read_buffer).await;

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
                state.t = Some(get_time(gps_result.time));
            }
            Ok(ParseResult::GLL(Some(gps_result))) => {
                info!("GLL location result parsed.");
                let mut state = STATE.lock().await;
                state.lt = Some(gps_result.latitude.as_f64() as f32);
                state.ln = Some(gps_result.longitude.as_f64() as f32);
                state.t = Some(get_time(gps_result.time));
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