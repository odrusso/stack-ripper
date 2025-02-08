#![deny(unsafe_code)]
#![no_main]
#![no_std]

use core::cell::RefCell;

use bleps::{
    ad_structure::{
        create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
    },
    async_attribute_server::AttributeServer,
    asynch::Ble,
    attribute_server::NotificationData,
    gatt,
};
use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_alloc as _;
use esp_backtrace as _;
use esp_hal::{rng::Rng, time, timer::timg::TimerGroup};
use esp_println::println;
use esp_wifi::{ble::controller::BleConnector, init, EspWifiController};

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());

    esp_alloc::heap_allocator!(72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);

    let init = &*mk_static!(
        EspWifiController<'static>,
        init(
            timg0.timer0,
            Rng::new(peripherals.RNG),
            peripherals.RADIO_CLK,
        )
        .unwrap()
    );

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timg1.timer0);

    let mut bluetooth = peripherals.BT;

    let connector = BleConnector::new(&init, &mut bluetooth);

    let now = || time::now().ticks();

    let mut ble = Ble::new(connector, now);
    println!("Connector created");

    loop {
        println!("{:?}", ble.init().await);
        println!("{:?}", ble.cmd_set_le_advertising_parameters().await);
        println!(
            "{:?}",
            ble.cmd_set_le_advertising_data(
                create_advertising_data(&[
                    AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                    AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x4201)]),
                    AdStructure::CompleteLocalName("oscartest123"),
                ])
                .unwrap()
            )
            .await
        );

        println!("{:?}", ble.cmd_set_le_advertise_enable(true).await);

        println!("started advertising");

        let mut some_counter = 0u8;

        let mut rf3 = |_offset: usize, data: &mut [u8]| {
            println!("trying to read");

            some_counter += 1;

            data[..5].copy_from_slice(&b"Holb!"[..]);
            data[6] = some_counter;
            7
        };

        gatt!([service {
            uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
            characteristics: [characteristic {
                name: "my_characteristic",
                uuid: "00000000-2354-11eb-9f10-fbc30a62cf38",
                read: rf3,
            },],
        },]);

        let mut rng = bleps::no_rng::NoRng;
        let mut srv = AttributeServer::new(&mut ble, &mut gatt_attributes, &mut rng);

        let counter = RefCell::new(0u8);
        let counter = &counter;

        let mut notifier = || async {
            Timer::after_millis(3_000).await;

            let mut data = [0u8; 13];
            data.copy_from_slice(b"Notification0");
            {
                let mut counter = counter.borrow_mut();
                data[data.len() - 1] += *counter;
                *counter = (*counter + 1) % 10;
            }
            NotificationData::new(my_characteristic_handle, &data)
        };

        srv.run(&mut notifier).await.unwrap();
    }
}
