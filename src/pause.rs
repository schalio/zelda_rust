// src/pause.rs

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::Duration;

const WINDOW_WIDTH:  u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

pub enum PauseResult {
    Resume,
    SaveAndResume,
    MainMenu,
    Quit,
}

pub fn pause_screen(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    event_pump: &mut sdl2::EventPump,
    ttf_context: &sdl2::ttf::Sdl2TtfContext,
) -> Result<PauseResult, String> {

    let font_title = ttf_context.load_font("assets/fonts/zelda.ttf", 28)
        .map_err(|e| e.to_string())?;
    let font_item = ttf_context.load_font("assets/fonts/zelda.ttf", 20)
        .map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();

    // Pré-rendu titre
    let title_surf = font_title.render("Pause")
        .blended(Color::RGB(255, 220, 50))
        .map_err(|e| e.to_string())?;
    let title_tex = texture_creator.create_texture_from_surface(&title_surf)
        .map_err(|e| e.to_string())?;
    let (title_w, title_h) = (title_tex.query().width, title_tex.query().height);

    let labels = ["Reprendre", "Sauvegarder", "Menu principal", "Quitter"];

    // Pré-rendu items normal + sélectionné
    let items_normal: Vec<(sdl2::render::Texture, u32, u32)> = labels.iter()
        .map(|l| {
            let s = font_item.render(l).blended(Color::RGB(160, 160, 160))
                .map_err(|e| e.to_string())?;
            let w = s.width(); let h = s.height();
            let t = texture_creator.create_texture_from_surface(&s)
                .map_err(|e| e.to_string())?;
            Ok((t, w, h))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let items_selected: Vec<(sdl2::render::Texture, u32, u32)> = labels.iter()
        .map(|l| {
            let s = font_item.render(l).blended(Color::RGB(255, 220, 50))
                .map_err(|e| e.to_string())?;
            let w = s.width(); let h = s.height();
            let t = texture_creator.create_texture_from_surface(&s)
                .map_err(|e| e.to_string())?;
            Ok((t, w, h))
        })
        .collect::<Result<Vec<_>, String>>()?;

    let mut selected: usize = 0;

    loop {
        let mut action: Option<usize> = None;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Ok(PauseResult::Quit),
                Event::KeyDown { keycode: Some(kc), .. } => match kc {
                    Keycode::Escape => return Ok(PauseResult::Resume),
                    Keycode::Up | Keycode::W => {
                        if selected > 0 { selected -= 1; }
                    }
                    Keycode::Down | Keycode::S => {
                        if selected + 1 < labels.len() { selected += 1; }
                    }
                    Keycode::Return | Keycode::Space => {
                        action = Some(selected);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if let Some(sel) = action {
            return Ok(match sel {
                0 => PauseResult::Resume,
                1 => PauseResult::SaveAndResume,
                2 => PauseResult::MainMenu,
                _ => PauseResult::Quit,
            });
        }

        // --- Rendu ---
        // Overlay semi-transparent par-dessus le jeu
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 160));
        canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
        canvas.fill_rect(Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT))?;
        canvas.set_blend_mode(sdl2::render::BlendMode::None);

        // Boîte centrale
        let box_w = 320u32;
        let box_h = 260u32;
        let box_x = (WINDOW_WIDTH  as i32 - box_w as i32) / 2;
        let box_y = (WINDOW_HEIGHT as i32 - box_h as i32) / 2;

        canvas.set_draw_color(Color::RGB(20, 20, 35));
        canvas.fill_rect(Rect::new(box_x, box_y, box_w, box_h))?;
        canvas.set_draw_color(Color::RGB(255, 220, 50));
        canvas.draw_rect(Rect::new(box_x, box_y, box_w, box_h))?;

        // Titre
        canvas.copy(&title_tex, None, Some(Rect::new(
            (WINDOW_WIDTH as i32 - title_w as i32) / 2,
            box_y + 20,
            title_w, title_h,
        )))?;

        // Items
        let start_y = box_y + 80;
        let spacing = 48i32;

        for i in 0..labels.len() {
            let items = if i == selected { &items_selected } else { &items_normal };
            let (ref tex, w, h) = items[i];

            let x = (WINDOW_WIDTH as i32 - w as i32) / 2;
            let y = start_y + i as i32 * spacing;
            canvas.copy(tex, None, Some(Rect::new(x, y, w, h)))?;

            // Curseur triangle
            if i == selected {
                let cx = x - 20;
                let cy = y;
                let ch = h as i32;
                canvas.set_draw_color(Color::RGB(255, 220, 50));
                for row in 0..ch {
                    let half = ch / 2;
                    let width = if row <= half {
                        (row * 8 / half).max(1)
                    } else {
                        ((ch - row) * 8 / half).max(1)
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