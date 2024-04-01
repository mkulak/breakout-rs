use std::error::Error;
use btleplug::platform::{Adapter, Manager, Peripheral};
use btleplug::api::{bleuuid::uuid_from_u16, Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType};
use tokio_stream::StreamExt;
use uuid::Uuid;
use crate::{Color, XY};

pub struct Display {
    peripheral: Peripheral,
    char: Characteristic
}


impl Display {
    pub async fn new() -> Result<Display, Box<dyn Error>> {
        let manager = Manager::new().await?;
        let adapters = manager.adapters().await?;
        let central = adapters.into_iter().nth(0).unwrap();
        let peripheral = find_device(central).await?;
        peripheral.connect().await?;
        peripheral.discover_services().await?;
        let write_uuid: Uuid = uuid_from_u16(0xfa02);
        let chars = peripheral.characteristics();
        let char = chars.iter().find(|c| c.uuid == write_uuid).unwrap().to_owned();
        Ok(Display { peripheral, char })
    }

    pub async fn set_pixel(&self, color: Color, xy: XY, wait: bool) {
        let cmd = vec![10, 0, 5, 1, 0, color.r, color.g, color.b, xy.x as u8, xy.y as u8];
        let write_type = if wait { WriteType::WithResponse } else { WriteType::WithoutResponse };
        let _ = self.peripheral.write(&self.char, &cmd, write_type).await;
    }
}

async fn find_device(central: Adapter) -> Result<Peripheral, Box<dyn Error>> {
    let mut events = central.events().await?;
    central.start_scan(ScanFilter::default()).await?;
    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(address) => {
                let peripheral = central.peripheral(&address).await.unwrap();
                let props = peripheral.properties().await.unwrap().unwrap();
                if props.local_name.iter().any(|name| name.contains("IDM-")) {
                    println!("Device found: {:?}", props);
                    println!("Address: {:?}", address);
                    return Ok(peripheral);
                }
            }
            _ => (),
        }
    }
    Err(Box::new(btleplug::Error::DeviceNotFound))
}
