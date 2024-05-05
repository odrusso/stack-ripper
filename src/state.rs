use defmt::Format;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use serde::{Deserialize, Serialize};

// Define and setup the system state
#[derive(Debug, Format, Serialize, Deserialize)]
pub struct State {
    pub ln: Option<f32>,  // GPS reported longitude
    pub lt: Option<f32>,  // GPS reported latitude
    pub ga: Option<f32>,  // GPS reported altitude, meters
    pub aaa: Option<f32>, // Altimeter-reported altitude absolute, meters
    pub aar: Option<f32>, // Altimeter-reported altitude reletive from starting height, delta meters
}

pub static STATE: Mutex<CriticalSectionRawMutex, State> = Mutex::new(State {
    ln: None,
    lt: None,
    ga: None,
    aaa: None,
    aar: None,
});
