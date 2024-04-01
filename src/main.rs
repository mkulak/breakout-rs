mod display;

use display::Display;
use std::error::Error;
use std::ops;
use std::time::Duration;

use btleplug::api::{bleuuid::uuid_from_u16, Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType};
use rand::{Rng, thread_rng};
use tokio::time;
use tokio_stream::StreamExt;
use uuid::Uuid;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let display = Display::new().await.unwrap();
    let mut game = Game::new();

    for y in 0..DIMENSION {
        for x in 0..DIMENSION {
            let color = if game.data[y][x] == 1 { COLOR_1 } else { COLOR_2 };
            display.set_pixel(color.clone(), XY { x, y }, true).await;
        }
    }

    loop {
        tick(&mut game, &display).await;
        time::sleep(Duration::from_millis(1)).await;
    }
}

async fn tick(game: &mut Game, display: &Display) {
    game.time += 1;
    update_ball(game, 0, display).await;
    update_ball(game, 1, display).await;
}

async fn update_ball(game: &mut Game, index: usize, display: &Display) {
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
            display.set_pixel(empty_color.clone(), XY { x: new_pos.x, y: pos.y }, true).await;
        }
    }
    if (!new_y_valid || game.data[new_pos.y][pos.x] == obstacle) {
        new_velocity.dy = -velocity.dy;
        if new_y_valid {
            game.data[new_pos.y][pos.x] = passable;
            display.set_pixel(empty_color.clone(), XY { x: pos.x, y: new_pos.y }, true).await;
        }
    }
    if new_velocity == *velocity && game.data[new_pos.y][new_pos.x] == obstacle {
        new_velocity.dx = -velocity.dx;
        new_velocity.dy = -velocity.dy;
        game.data[new_pos.y][new_pos.x] = passable;
        display.set_pixel(empty_color.clone(), new_pos, true).await;
    }
    let old_pos = pos.clone();
    *velocity = new_velocity;
    *pos = pos.clone() + velocity.clone();
    display.set_pixel(fill_color.clone(), pos.clone(), true).await;
    display.set_pixel(empty_color.clone(), old_pos, true).await;
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
