// src/screens.rs

use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::Duration;

const WINDOW_WIDTH:  u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

pub fn game_over_screen(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    texture_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    ttf_context: &sdl2::ttf::Sdl2TtfContext,
    event_pump: &mut sdl2::EventPump,
) -> Result<(), String> {

    let font_large = ttf_context
        .load_font("assets/fonts/zelda.ttf", 32)
        .map_err(|e| e.to_string())?;
    let font_small = ttf_context
        .load_font("assets/fonts/zelda.ttf", 12)
        .map_err(|e| e.to_string())?;

    let title_surf = font_large.render("GAME OVER")
        .blended(Color::RGB(200, 40, 40))
        .map_err(|e| e.to_string())?;
    let sub_surf = font_small.render("Appuyez sur une touche...")
        .blended(Color::RGB(180, 180, 180))
        .map_err(|e| e.to_string())?;

    let mut title_tex = texture_creator.create_texture_from_surface(title_surf)
        .map_err(|e| e.to_string())?;
    let mut sub_tex = texture_creator.create_texture_from_surface(sub_surf)
        .map_err(|e| e.to_string())?;
    let mut img_tex = texture_creator
        .load_texture("assets/sprites/game_over.png")
        .map_err(|e| e.to_string())?;

    let (tw, th) = (title_tex.query().width, title_tex.query().height);
    let (sw, sh) = (sub_tex.query().width,   sub_tex.query().height);
    let (iw, ih) = (img_tex.query().width,   img_tex.query().height);

    let img_x = (WINDOW_WIDTH  as i32 - iw as i32) / 2;
    let img_y = (WINDOW_HEIGHT as i32 / 2) - ih as i32 - th as i32 - 30;

    let mut alpha: f32 = 0.0;
    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
    title_tex.set_blend_mode(sdl2::render::BlendMode::Blend);
    sub_tex.set_blend_mode(sdl2::render::BlendMode::Blend);
    img_tex.set_blend_mode(sdl2::render::BlendMode::Blend);

    'go_loop: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'go_loop,
                Event::KeyDown { .. } | Event::MouseButtonDown { .. } => {
                    if alpha >= 250.0 { break 'go_loop; }
                }
                _ => {}
            }
        }

        alpha = (alpha + 3.0).min(255.0);
        let a = alpha as u8;

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        img_tex.set_alpha_mod(a);
        canvas.copy(&img_tex, None, Some(Rect::new(img_x, img_y, iw, ih)))?;

        title_tex.set_alpha_mod(a);
        let tx = (WINDOW_WIDTH  as i32 - tw as i32) / 2;
        let ty = (WINDOW_HEIGHT as i32 / 2) - th as i32 - 10;
        canvas.copy(&title_tex, None, Some(Rect::new(tx, ty, tw, th)))?;

        if alpha >= 255.0 {
            sub_tex.set_alpha_mod(255);
            let sx = (WINDOW_WIDTH  as i32 - sw as i32) / 2;
            let sy = ty + th as i32 + 20;
            canvas.copy(&sub_tex, None, Some(Rect::new(sx, sy, sw, sh)))?;
        }

        canvas.present();
        std::thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}