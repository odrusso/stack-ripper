#![deny(unsafe_code)]
#![no_main]
#![no_std]

use bleps::ad_structure::{
    create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
};
use bleps::async_attribute_server::AttributeServer;
use bleps::asynch::Ble;
use bleps::att::Uuid;
use bleps::attribute_server::NotificationData;
use bleps::{gatt, HciConnector};
use embassy_executor::Spawner;

use esp_hal::{peripherals::Peripherals, time, timer::timg::TimerGroup};

use defmt::{error, info};
use esp_backtrace as _;
use esp_hal::rng::Rng;
use esp_wifi::ble::controller::BleConnector;
use esp_wifi::init;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> () {
    info!("Initializing");

    let peripherals: Peripherals = esp_hal::init(esp_hal::Config::default());
    let timg0 = TimerGroup::new(peripherals.TIMG0);

    esp_alloc::heap_allocator!(72 * 1024);

    info!("Initializing compete");

    let init = esp_wifi::init(
        timg0.timer0,
        Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let now = || time::now().duration_since_epoch().to_millis();

    let mut bluetooth = peripherals.BT;

    let connector = BleConnector::new(&init, &mut bluetooth);
    let mut ble = Ble::new(connector, now);

    loop {
        ble.init().await.unwrap();
        ble.cmd_set_le_advertising_parameters().await.unwrap();

        ble.cmd_set_le_advertising_data(
            create_advertising_data(&[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1810)]),
                AdStructure::CompleteLocalName("oscar esp32"),
            ])
            .unwrap(),
        )
        .await
        .unwrap();

        ble.cmd_set_le_advertise_enable(true).await.unwrap();

        info!("Started advertising");

        let mut wf2 = |offset: usize, data: &[u8]| {
            info!("RECEIVED2: Offset {}, data {}", offset, data);
        };

        let mut index_array: usize = 0;
        let data_array: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

        let mut rf = |_offset: usize, data: &mut [u8]| {
            index_array += 1;
            *data_array.get(index_array).unwrap()
        };

        let mut rf2 = |_offset: usize, data: &mut [u8]| *data_array.get(1).unwrap();

        gatt!([service {
            uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
            characteristics: [
                characteristic {
                    uuid: "947312e0-2354-11eb-9f10-fbc30a62cf38",
                    name: "test_oscar",
                    read: rf,
                    write: wf2
                },
                characteristic {
                    uuid: "957312e0-2354-11eb-9f10-fbc30a62cf38",
                    name: "testoscartwo",
                    description: "somesuch description",
                    read: rf2,
                }
            ]
        }]);

        let mut rng = bleps::no_rng::NoRng;
        let mut srv = AttributeServer::new(&mut ble, &mut gatt_attributes, &mut rng);

        // srv.update_le_advertising_data([]])

        info!("Att srv setup");

        let mut notifier = || async {
            let mut data = [0u8; 13];
            NotificationData::new(0, &data)
        };

        srv.run(&mut notifier).await.unwrap();
    }
}
