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

    let mut game = Game {
        data: [[0; DIMENSION]; DIMENSION],
        ball1: XY { x: 0, y: rng.gen_range((DIMENSION / 2 - DIMENSION / 3)..(DIMENSION / 2 + DIMENSION / 3)) as i8 },
        ball2: XY { x: (DIMENSION - 1) as i8, y: rng.gen_range((DIMENSION / 2 - DIMENSION / 3)..(DIMENSION / 2 + DIMENSION / 3)) as i8 },
        ball_speed_1: XY { x: if rng.gen_bool(0.5) { 1 } else { -1 }, y: if rng.gen_bool(0.5) { 1 } else { -1 } },
        ball_speed_2: XY { x: if rng.gen_bool(0.5) { 1 } else { -1 }, y: if rng.gen_bool(0.5) { 1 } else { -1 } },
        time: 0,
    };
    for y in 0..DIMENSION {
        for x in DIMENSION / 2..DIMENSION {
            game.data[y][x] = 1
        }
    }
    for y in 0..DIMENSION {
        for x in 0..DIMENSION {
            let color = if game.data[y][x] == 1 { COLOR_1 } else { COLOR_2 };
            set_pixel(&peripheral, &cmd_char, color.clone(), XY { x: x as i8, y: y as i8 }, true).await;
            // time::sleep(Duration::from_millis(1)).await;
        }
        time::sleep(Duration::from_millis(50)).await;
    }

    loop {
        tick(&mut game, &peripheral, &cmd_char).await;
        time::sleep(Duration::from_millis(1)).await;
    }
}

async fn tick(game: &mut Game, p: &Peripheral, c: &Characteristic) {
    game.time += 1;
    update_ball(game, true, p, c).await;
    update_ball(game, false, p, c).await;
}

async fn update_ball(game: &mut Game, first: bool, p: &Peripheral, c: &Characteristic) {
    let passable: u8 = if first { 0 } else { 1 };
    let obstacle: u8 = if first { 1 } else { 0 };
    let ball: &mut XY = if first { &mut game.ball1 } else { &mut game.ball2 };
    let ball_speed: &mut XY = if first { &mut game.ball_speed_1 } else { &mut game.ball_speed_2 };
    let fill_color = if first { COLOR_1 } else { COLOR_2 };
    let empty_color = if first { COLOR_2 } else { COLOR_1 };
    let new_x = (ball.x + ball_speed.x) as usize;
    let new_y = (ball.y + ball_speed.y) as usize;
    let new_x_valid = new_x >= 0 && new_x < DIMENSION;
    let new_y_valid = new_y >= 0 && new_y < DIMENSION;
    let mut new_dx = ball_speed.x;
    let mut new_dy = ball_speed.y;
    if (!new_x_valid || game.data[ball.y as usize][new_x] == obstacle) {
        new_dx = -ball_speed.x;
        if new_x_valid {
            game.data[ball.y as usize][new_x] = passable;
            set_pixel(p, c, empty_color.clone(), XY { x: new_x as i8, y: ball.y }, true).await;
        }
    }
    if (!new_y_valid || game.data[new_y][ball.x as usize] == obstacle) {
        new_dy = -ball_speed.y;
        if new_y_valid {
            game.data[new_y][ball.x as usize] = passable;
            set_pixel(p, c, empty_color.clone(), XY { x: ball.x, y: new_y as i8 }, true).await;
        }
    }
    if new_dx == ball_speed.x && new_dy == ball_speed.y && game.data[new_y][new_x] == obstacle {
        new_dx = -ball_speed.x;
        new_dy = -ball_speed.y;
        game.data[new_y][new_x] = passable;
        set_pixel(p, c, empty_color.clone(), XY { x: new_x as i8, y: new_y as i8 }, true).await;
    }
    ball_speed.x = new_dx;
    ball_speed.y = new_dy;
    let old_pos = ball.clone();
    ball.x += ball_speed.x;
    ball.y += ball_speed.y;
    set_pixel(p, c, fill_color.clone(), ball.clone(), true).await;
    set_pixel(p, c, empty_color.clone(), old_pos, true).await;
}

async fn set_pixel(p: &Peripheral, c: &Characteristic, color: Color, xy: XY, wait: bool) {
    let cmd = vec![10, 0, 5, 1, 0, color.r, color.g, color.b, xy.x as u8, xy.y as u8];
    let write_type = if wait { WriteType::WithResponse } else { WriteType::WithoutResponse };
    // let _ = p.write(c, &cmd, WriteType::WithoutResponse).await;
    let _ = p.write(c, &cmd, write_type).await;
    // time::sleep(Duration::from_millis(1)).await;
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

#[derive(Clone, Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Clone, Debug)]
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
const COLOR_1: Color = Color { r: 30, g: 255, b: 30 };
const COLOR_2: Color = Color { r: 255, g: 50, b: 50 };
