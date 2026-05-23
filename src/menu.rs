// src/menu.rs

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::{Duration, Instant};

use crate::game::AppState;

const WINDOW_WIDTH:  u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

pub fn main_menu(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    event_pump: &mut sdl2::EventPump,
    ttf_context: &sdl2::ttf::Sdl2TtfContext,
) -> Result<AppState, String> {

    let font_title = ttf_context
        .load_font("assets/fonts/zelda.ttf", 48)
        .map_err(|e| e.to_string())?;

    let font_item = ttf_context
        .load_font("assets/fonts/zelda.ttf", 22)
        .map_err(|e| e.to_string())?;

    let items = ["Jouer", "Quitter"];
    let mut selected: usize = 0;

    // Pré-rendu du titre
    let title_surf = font_title
        .render("Zelda-like Rust")
        .blended(Color::RGB(255, 220, 50))
        .map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();
    let title_tex = texture_creator
        .create_texture_from_surface(&title_surf)
        .map_err(|e| e.to_string())?;
    let title_w = title_tex.query().width;
    let title_h = title_tex.query().height;

    let mut blink_timer: f32 = 0.0;
    let mut last_time = Instant::now();

    loop {
        let now = Instant::now();
        let dt  = now.duration_since(last_time).as_secs_f32();
        last_time = now;
        blink_timer = (blink_timer + dt) % 1.0;

        // --- Événements ---
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Ok(AppState::Quit),
                Event::KeyDown { keycode: Some(kc), .. } => match kc {
                    Keycode::Escape => return Ok(AppState::Quit),
                    Keycode::Up   | Keycode::W => {
                        selected = selected.saturating_sub(1);
                    }
                    Keycode::Down | Keycode::S => {
                        if selected + 1 < items.len() { selected += 1; }
                    }
                    Keycode::Return | Keycode::Space => {
                        return Ok(if selected == 0 {
                            AppState::Play
                        } else {
                            AppState::Quit
                        });
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // --- Rendu ---
        canvas.set_draw_color(Color::RGB(10, 10, 20));
        canvas.clear();

        // Titre
        let tx = (WINDOW_WIDTH as i32 - title_w as i32) / 2;
        let ty = WINDOW_HEIGHT as i32 / 4;
        canvas.copy(&title_tex, None, Some(Rect::new(tx, ty, title_w, title_h)))?;

        // Items du menu
        let item_start_y = WINDOW_HEIGHT as i32 / 2;
        let item_spacing  = 50i32;

        for (i, label) in items.iter().enumerate() {
            let color = if i == selected {
                Color::RGB(255, 220, 50)
            } else {
                Color::RGB(160, 160, 160)
            };

            let surf = font_item
                .render(label)
                .blended(color)
                .map_err(|e| e.to_string())?;
            let tex = texture_creator
                .create_texture_from_surface(&surf)
                .map_err(|e| e.to_string())?;
            let iw = tex.query().width;
            let ih = tex.query().height;
            let ix = (WINDOW_WIDTH as i32 - iw as i32) / 2;
            let iy = item_start_y + i as i32 * item_spacing;
            canvas.copy(&tex, None, Some(Rect::new(ix, iy, iw, ih)))?;

            // Curseur triangle sur l'item sélectionné
            if i == selected && blink_timer < 0.6 {
                let cw: i32 = 10;
                let ch: i32 = ih as i32;
                let cx: i32 = ix - cw - 12;
                let cy: i32 = iy;

                canvas.set_draw_color(Color::RGB(255, 220, 50));
                for row in 0..ch {
                    let half = ch / 2;
                    let width = if row <= half {
                        (row * cw / half).max(1)
                    } else {
                        ((ch - row) * cw / half).max(1)
                    };
                    canvas.draw_line(
                        sdl2::rect::Point::new(cx, cy + row),
                        sdl2::rect::Point::new(cx + width, cy + row),
                    )?;
                }
            }
        }

        canvas.present();
        std::thread::sleep(Duration::from_millis(16));
    }
}
