use std::error::Error;
use std::ops;
use std::time::Duration;

use btleplug::api::{Central, Manager as _, Peripheral as _};
use circular_buffer::CircularBuffer;
use rand::{Rng, thread_rng};
use tokio::time;
use tokio_stream::StreamExt;

use display::Display;

mod display;

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
    let passable = if index == 0 { 0 } else { 1 };
    let obstacle = if index == 0 { 1 } else { 0 };
    let fill_color = if index == 0 { COLOR_1 } else { COLOR_2 };
    let empty_color = if index == 0 { COLOR_2 } else { COLOR_1 };
    let pos = *game.balls.get(index).unwrap();
    let velocity = *game.velocities.get(index).unwrap();
    let new_pos = pos + velocity;
    let new_x_valid = new_pos.x >= 0 && new_pos.x < DIMENSION;
    let new_y_valid = new_pos.y >= 0 && new_pos.y < DIMENSION;
    let mut new_velocity = velocity;
    if (!new_x_valid) {
        new_velocity.dx = -velocity.dx;
    }
    if (!new_y_valid) {
        new_velocity.dy = -velocity.dy;
    }
    if (new_x_valid && new_y_valid) {
        let next_x = pos.with_x(new_pos.x);
        let next_y = pos.with_y(new_pos.y);
        let collision_x = game.get(next_x) == obstacle;
        let collision_y = game.get(next_y) == obstacle;
        let collision_xy = game.get(new_pos) == obstacle;
        if collision_x {
            game.set(next_x, passable);
            display.set_pixel(empty_color, next_x, true).await;
        }
        if collision_y {
            game.set(next_y, passable);
            display.set_pixel(empty_color, next_y, true).await;
        }
        if collision_xy {
            game.set(new_pos, passable);
            display.set_pixel(empty_color, new_pos, true).await;
        }
        if collision_x || collision_xy {
            new_velocity.dx = -velocity.dx;
        }
        if collision_y || collision_xy {
            new_velocity.dy = -velocity.dy;
        }
        if index == 0 && velocity != new_velocity {
            let state = calc_state(pos, velocity, collision_x, collision_y, collision_xy);
            if game.prev_states.iter().any(|prev| state == *prev) {
                println!("Collision detected: {:?}", state);
                if thread_rng().gen_bool(0.5) {
                    new_velocity.dx = -new_velocity.dx
                } else {
                    new_velocity.dy = -new_velocity.dy
                }
            }
            game.prev_states.push_back(state);
        }
    }
    *game.velocities.get_mut(index).unwrap() = new_velocity;
    *game.balls.get_mut(index).unwrap() = pos + new_velocity;
    display.set_pixel(fill_color, pos + new_velocity, true).await;
    display.set_pixel(empty_color, pos, true).await;
}

fn calc_state(pos: XY, velocity: Velocity, cx: bool, cy: bool, cxy: bool) -> u32 {
    ((pos.x as u32) << 24)
        .wrapping_add((pos.y as u32) << 16)
        .wrapping_add((velocity.dx as u32) << 8)
        .wrapping_add(velocity.dy as u32)
        .wrapping_add(if cx { 1 } else { 0 })
        .wrapping_add(if cy { 1 } else { 0 })
        .wrapping_add(if cxy { 1 } else { 0 })
}

#[derive(Copy, Clone, Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Copy, Clone, Debug)]
struct XY {
    x: usize,
    y: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct Velocity {
    dx: i8,
    dy: i8,
}

struct Game {
    data: [[u8; DIMENSION]; DIMENSION],
    balls: [XY; 2],
    velocities: [Velocity; 2],
    time: u32,
    prev_states: CircularBuffer<8, u32>
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

impl XY {
    fn with_x(&self, new_x: usize) -> Self {
        XY { x: new_x, y: self.y }
    }
    fn with_y(&self, new_y: usize) -> Self {
        XY { x: self.x, y: new_y }
    }
}

impl Game {
    fn new() -> Self {
        let mut rng = thread_rng();
        let mut data = [[0; DIMENSION]; DIMENSION];
        // let balls = [
        //     XY { x: 0, y: DIMENSION / 2 },
        //     XY { x: DIMENSION - 1, y: DIMENSION / 2 }
        // ];
        let balls = [
            XY { x: 0, y: rng.gen_range((DIMENSION / 2 - DIMENSION / 3)..(DIMENSION / 2 + DIMENSION / 3)) },
            XY { x: DIMENSION - 1, y: rng.gen_range((DIMENSION / 2 - DIMENSION / 3)..(DIMENSION / 2 + DIMENSION / 3)) }
        ];
        // let velocities = [
        //     Velocity { dx: 1, dy: 1 },
        //     Velocity { dx: -1, dy: 1 }
        // ];
        let velocities = [
            Velocity { dx: random_dir(), dy: random_dir() },
            Velocity { dx: random_dir(), dy: random_dir() }
        ];
        for y in 0..DIMENSION {
            for x in DIMENSION / 2..DIMENSION {
                data[y][x] = 1;
            }
        }
        Game { data, balls, velocities, time: 0, prev_states: CircularBuffer::new() }
    }

    fn get(&self, pos: XY) -> u8 {
        self.data[pos.y][pos.x]
    }

    fn set(&mut self, pos: XY, value: u8) {
        self.data[pos.y][pos.x] = value
    }
}
