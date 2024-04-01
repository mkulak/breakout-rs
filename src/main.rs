use std::error::Error;
use std::ops;
use std::time::Duration;

use btleplug::api::{bleuuid::uuid_from_u16, Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Adapter, Manager, Peripheral};
use rand::{Rng, thread_rng};
use tokio::time;
use tokio_stream::StreamExt;
use uuid::Uuid;

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

    let mut game = Game::new();

    for y in 0..DIMENSION {
        for x in 0..DIMENSION {
            let color = if game.data[y][x] == 1 { COLOR_1 } else { COLOR_2 };
            set_pixel(&peripheral, &cmd_char, color.clone(), XY { x, y }, true).await;
        }
    }

    loop {
        tick(&mut game, &peripheral, &cmd_char).await;
        time::sleep(Duration::from_millis(1)).await;
    }
}

async fn tick(game: &mut Game, p: &Peripheral, c: &Characteristic) {
    game.time += 1;
    update_ball(game, 0, p, c).await;
    update_ball(game, 1, p, c).await;
}

async fn update_ball(game: &mut Game, index: usize, p: &Peripheral, c: &Characteristic) {
    let passable: u8 = if index == 0 { 0 } else { 1 };
    let obstacle: u8 = if index == 0 { 1 } else { 0 };
    let fill_color = if index == 0 { COLOR_1 } else { COLOR_2 };
    let empty_color = if index == 0 { COLOR_2 } else { COLOR_1 };
    let pos = game.balls.get_mut(index).unwrap();
    let velocity = game.velocities.get_mut(index).unwrap();
    let new_pos = pos.clone() + velocity.clone();
    let new_x_valid = new_pos.x >= 0 && new_pos.x < DIMENSION;
    let new_y_valid = new_pos.y >= 0 && new_pos.y < DIMENSION;
    let mut new_velocity = velocity.clone();
    if (!new_x_valid || game.data[pos.y][new_pos.x] == obstacle) {
        new_velocity.dx = -velocity.dx;
        if new_x_valid {
            game.data[pos.y][new_pos.x] = passable;
            set_pixel(p, c, empty_color.clone(), XY { x: new_pos.x, y: pos.y }, true).await;
        }
    }
    if (!new_y_valid || game.data[new_pos.y][pos.x] == obstacle) {
        new_velocity.dy = -velocity.dy;
        if new_y_valid {
            game.data[new_pos.y][pos.x] = passable;
            set_pixel(p, c, empty_color.clone(), XY { x: pos.x, y: new_pos.y }, true).await;
        }
    }
    if new_velocity == *velocity && game.data[new_pos.y][new_pos.x] == obstacle {
        new_velocity.dx = -velocity.dx;
        new_velocity.dy = -velocity.dy;
        game.data[new_pos.y][new_pos.x] = passable;
        set_pixel(p, c, empty_color.clone(), new_pos, true).await;
    }
    let old_pos = pos.clone();
    *velocity = new_velocity;
    *pos = pos.clone() + velocity.clone();
    set_pixel(p, c, fill_color.clone(), pos.clone(), true).await;
    set_pixel(p, c, empty_color.clone(), old_pos, true).await;
}

async fn set_pixel(p: &Peripheral, c: &Characteristic, color: Color, xy: XY, wait: bool) {
    let cmd = vec![10, 0, 5, 1, 0, color.r, color.g, color.b, xy.x as u8, xy.y as u8];
    let write_type = if wait { WriteType::WithResponse } else { WriteType::WithoutResponse };
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
    x: usize,
    y: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Velocity {
    dx: i8,
    dy: i8,
}

struct Game {
    data: [[u8; DIMENSION]; DIMENSION],
    balls: [XY; 2],
    velocities: [Velocity; 2],
    time: u32,
}

const DIMENSION: usize = 32;
const COLOR_1: Color = Color { r: 30, g: 255, b: 30 };
const COLOR_2: Color = Color { r: 255, g: 50, b: 50 };

impl ops::Add<Velocity> for XY {
    type Output = XY;

    fn add(self, rhs: Velocity) -> Self::Output {
        XY { x: (self.x as i8 + rhs.dx) as usize, y: (self.y as i8 + rhs.dy) as usize }
    }
}

fn random_dir() -> i8 {
    if thread_rng().gen_bool(0.5) { 1 } else { -1 }
}

impl Game {
    fn new() -> Game {
        let mut rng = thread_rng();
        let mut data = [[0; DIMENSION]; DIMENSION];
        let balls = [
            XY { x: 0, y: rng.gen_range((DIMENSION / 2 - DIMENSION / 3)..(DIMENSION / 2 + DIMENSION / 3)) },
            XY { x: DIMENSION - 1, y: rng.gen_range((DIMENSION / 2 - DIMENSION / 3)..(DIMENSION / 2 + DIMENSION / 3)) }
        ];
        let velocities = [
            Velocity { dx: random_dir(), dy: random_dir() },
            Velocity { dx: random_dir(), dy: random_dir() }
        ];
        for y in 0..DIMENSION {
            for x in DIMENSION / 2..DIMENSION {
                data[y][x] = 1;
            }
        }
        Game { data, balls, velocities, time: 0 }
    }
}
