use btleplug::api::{bleuuid::uuid_from_u16, Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use rand::{Rng, thread_rng};
use std::error::Error;
use std::thread;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

// "0000fa02-0000-1000-8000-00805f9b34fb"
// "0x00000000_0000_1000_8000_00805f9b34fb"
const WRITE_UUID: Uuid = uuid_from_u16(0xfa02);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await.unwrap();

    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().nth(0).unwrap();

    central.start_scan(ScanFilter::default()).await?;
    // instead of waiting, you can use central.events() to get a stream which will
    // notify you of new devices, for an example of that see examples/event_driven_discovery.rs
    time::sleep(Duration::from_secs(2)).await;

    // find the device we're interested in
    let device = find_light(&central).await.unwrap();
    println!("address: {:?}", device.address());
    device.connect().await?;
    device.discover_services().await?;

    let chars = device.characteristics();
    let cmd_char = chars.iter().find(|c| c.uuid == WRITE_UUID).unwrap();
    println!("{:?} ", cmd_char);

    let mut rng = thread_rng();
    for x in 0..20 {
        //                             r    g    b    x  y
        let cmd = vec![10, 0, 5, 1, 0, 128, 255, 128, x, 1];
        device.write(&cmd_char, &cmd, WriteType::WithoutResponse).await?;
        time::sleep(Duration::from_millis(50)).await;
    }
    Ok(())
}

async fn find_light(central: &Adapter) -> Option<Peripheral> {
    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.contains("IDM-"))
        {
            return Some(p);
        }
    }
    None
}