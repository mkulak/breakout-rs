use std::error::Error;
use std::time::Duration;

use btleplug::api::{bleuuid::uuid_from_u16, Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use rand::{Rng, thread_rng};
use tokio::time;
use tokio_stream::StreamExt;
use uuid::Uuid;

// "0000fa02-0000-1000-8000-00805f9b34fb"
// "0x00000000_0000_1000_8000_00805f9b34fb"
const WRITE_UUID: Uuid = uuid_from_u16(0xfa02);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;

    let adapters = manager.adapters().await?;
    let central = adapters.into_iter().nth(0).unwrap();

    let peripheral = find_device(central).await?;
    peripheral.connect().await?;
    peripheral.discover_services().await?;

    let chars = peripheral.characteristics();
    let cmd_char = chars.iter().find(|c| c.uuid == WRITE_UUID).unwrap();

    let mut rng = thread_rng();
    for y in 0..DIMENSION {
        for x in 0..DIMENSION {
            let cmd = vec![10, 0, 5, 1, 0, rng.gen_range(0..255), 128, 128, x as u8, y as u8];
            peripheral.write(&cmd_char, &cmd, WriteType::WithoutResponse).await?;
            time::sleep(Duration::from_millis(5)).await;
        }
    }
    time::sleep(Duration::from_millis(50)).await;

    let mut game = Game {
        data: [[0; DIMENSION]; DIMENSION],
        ball1: XY { x: 0, y: rng.gen_range((DIMENSION / 2 - DIMENSION / 3)..(DIMENSION / 2 + DIMENSION / 3)) as i8 },
        ball2: XY { x: (DIMENSION - 1) as i8, y: rng.gen_range((DIMENSION / 2 - DIMENSION / 3)..(DIMENSION / 2 + DIMENSION / 3)) as i8 },
        ball_speed_1: XY { x: if rng.gen_bool(0.5) { 1 } else { -1 }, y: if rng.gen_bool(0.5) { 1 } else { -1 } },
        ball_speed_2: XY { x: if rng.gen_bool(0.5) { 1 } else { -1 }, y: if rng.gen_bool(0.5) { 1 } else { -1 } },
        time: 0
    };
    loop {
        time::sleep(Duration::from_millis(50)).await;
        tick(&mut game);
        paint(&game);
    }

    Ok(())
}

fn tick(game: &mut Game) {

}

fn paint(game: &Game) {

}

async fn set_pixel(p: &Peripheral, c: &Characteristic, color: Color, xy: XY) {
    let cmd = vec![10, 0, 5, 1, 0, color.r, color.g, color.b, xy.x as u8, xy.y as u8];
    p.write(c, &cmd, WriteType::WithoutResponse).await;
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

struct Color {
    r: u8,
    g: u8,
    b: u8,
}

struct XY {
    x: i8,
    y: i8,
}

struct Game {
    data: [[u8; DIMENSION]; DIMENSION],
    ball1: XY,
    ball_speed_1: XY,
    ball2: XY,
    ball_speed_2: XY,
    time: u32,
}

const DIMENSION: usize = 32;
