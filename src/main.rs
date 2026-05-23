// src/main.rs

mod camera;
mod tilemap;
mod player;
mod enemy;
mod enemy_type;
pub mod tile_properties;
pub mod map_loader;
pub mod combat;
pub mod hud;
pub mod audio;
pub mod objects;
pub mod transition;
pub mod npc;
pub mod game;
pub mod map_utils;
pub mod utils;
pub mod screens;
pub mod rendering;
pub mod run;
pub mod menu;
pub mod sdl_context;

use crate::game::AppState;
use sdl_context::SdlBundle;

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;


fn main() -> Result<(), String> {
    let mut sdl = SdlBundle::init("Zelda-like Rust", WINDOW_WIDTH, WINDOW_HEIGHT)?;

    loop {
        match menu::main_menu(&mut sdl.canvas, &mut sdl.event_pump, &sdl.ttf_context)? {
            AppState::Play => {
                run::run(&mut sdl.canvas, &mut sdl.event_pump, &sdl.ttf_context)?;
            }
            AppState::Quit => break,
        }
    }

    Ok(())

}
