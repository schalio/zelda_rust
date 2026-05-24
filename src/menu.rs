// src/menu.rs

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::image::LoadTexture;
use std::time::{Duration, Instant};

use crate::save::{list_slots, SaveData};
use crate::game::AppState;

const WINDOW_WIDTH:  u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const MENU_TITLE: &str = "La Légende de Zelda";
const MENU_BG: &str = "assets/sprites/menu_bg.png";

pub enum MenuResult {
    NewGame(u8),          // slot choisi pour nouvelle partie
    Continue(u8, SaveData), // slot + données
    Quit,
}


pub fn main_menu(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    event_pump: &mut sdl2::EventPump,
    ttf_context: &sdl2::ttf::Sdl2TtfContext,
) -> Result<AppState, String> {

    let font_title = ttf_context
        .load_font("assets/fonts/zelda.ttf", 36)
        .map_err(|e| e.to_string())?;

    let font_item = ttf_context
        .load_font("assets/fonts/zelda.ttf", 22)
        .map_err(|e| e.to_string())?;

    let items = ["Jouer", "Quitter"];
    let mut selected: usize = 0;

    let texture_creator = canvas.texture_creator();
    let bg_tex = texture_creator.load_texture(MENU_BG)?;

    // Pré-rendu du titre
    let title_surf = font_title
        .render(MENU_TITLE)
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
        // canvas.set_draw_color(Color::RGB(10, 10, 20));
        // canvas.clear();

        // Fond centré avec proportions conservées
        let bg_query = bg_tex.query();
        let bg_w = bg_query.width as f32;
        let bg_h = bg_query.height as f32;

        let scale = (WINDOW_WIDTH as f32 / bg_w).min(WINDOW_HEIGHT as f32 / bg_h);
        let dst_w = (bg_w * scale) as u32;
        let dst_h = (bg_h * scale) as u32;
        let dst_x = (WINDOW_WIDTH  as i32 - dst_w as i32) / 2;
        let dst_y = (WINDOW_HEIGHT as i32 - dst_h as i32) / 2;

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.copy(&bg_tex, None, Some(Rect::new(dst_x, dst_y, dst_w, dst_h)))?;

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


pub fn select_slot(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    event_pump: &mut sdl2::EventPump,
    ttf_context: &sdl2::ttf::Sdl2TtfContext,
) -> Result<MenuResult, String> {

    let font = ttf_context.load_font("assets/fonts/zelda.ttf", 14)
        .map_err(|e| e.to_string())?;
    let font_small = ttf_context.load_font("assets/fonts/zelda.ttf", 10)
        .map_err(|e| e.to_string())?;

    let slots = list_slots();
    let mut selected: usize = 0;
    // 0 = Nouveau, 1-3 = slots, 4 = Quitter
    let item_count = 5;

    loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Ok(MenuResult::Quit),
                Event::KeyDown { keycode: Some(kc), .. } => match kc {
                    Keycode::Escape => return Ok(MenuResult::Quit),
                    Keycode::Up | Keycode::W => {
                        if selected > 0 { selected -= 1; }
                    }
                    Keycode::Down | Keycode::S => {
                        if selected + 1 < item_count { selected += 1; }
                    }
                    Keycode::Return | Keycode::Space => {
                        match selected {
                            0 => {
                                // Nouvelle partie : choisir le slot libre ou écraser
                                let free_slot = slots.iter().position(|s| s.is_none())
                                    .map(|i| i as u8 + 1)
                                    .unwrap_or(1);
                                return Ok(MenuResult::NewGame(free_slot));
                            }
                            1..=3 => {
                                let slot = selected as u8;
                                if let Some(ref data) = slots[selected - 1] {
                                    return Ok(MenuResult::Continue(slot, data.clone()));
                                } else {
                                    return Ok(MenuResult::NewGame(slot));
                                }
                            }
                            _ => return Ok(MenuResult::Quit),
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGB(10, 10, 20));
        canvas.clear();

        let labels = ["Nouvelle partie", "Slot 1", "Slot 2", "Slot 3", "Quitter"];
        let start_y = 80i32;
        let spacing = 90i32;
        let texture_creator = canvas.texture_creator();

        for (i, label) in labels.iter().enumerate() {
            let color = if i == selected {
                Color::RGB(255, 220, 50)
            } else {
                Color::RGB(160, 160, 160)
            };

            // Titre du slot
            let surf = font.render(label).blended(color).map_err(|e| e.to_string())?;
            let tex  = texture_creator.create_texture_from_surface(&surf).map_err(|e| e.to_string())?;
            let (w, h) = (tex.query().width, tex.query().height);
            let x = (WINDOW_WIDTH as i32 - w as i32) / 2;
            let y = start_y + i as i32 * spacing;
            canvas.copy(&tex, None, Some(Rect::new(x, y, w, h)))?;

            // Infos du slot (si existe)
            if i >= 1 && i <= 3 {
                let info = if let Some(ref data) = slots[i - 1] {
                    format!("Map: {}   Rubis: {}", data.current_map, data.rubies)
                } else {
                    "(vide)".to_string()
                };
                let info_surf = font_small.render(&info)
                    .blended(Color::RGB(120, 120, 120))
                    .map_err(|e| e.to_string())?;
                let info_tex = texture_creator
                    .create_texture_from_surface(&info_surf)
                    .map_err(|e| e.to_string())?;
                let (iw, ih) = (info_tex.query().width, info_tex.query().height);
                let ix = (WINDOW_WIDTH as i32 - iw as i32) / 2;
                canvas.copy(&info_tex, None, Some(Rect::new(ix, y + h as i32 + 4, iw, ih)))?;
            }
        }

        canvas.present();
        std::thread::sleep(Duration::from_millis(16));
    }
}