// src/rendering.rs

use sdl2::pixels::Color;
use sdl2::rect::Rect;

const WINDOW_WIDTH:  u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

pub fn render_dialogue_box(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    texture_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    font: &sdl2::ttf::Font,
    text: &str,
    is_last: bool,
) -> Result<(), String> {
    let box_x = 32i32;
    let box_h = 120i32;
    let box_y = WINDOW_HEIGHT as i32 - box_h - 24;
    let box_w = WINDOW_WIDTH - 64;

    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(0, 0, 0, 220));
    canvas.fill_rect(Rect::new(box_x, box_y, box_w, box_h as u32))?;
    canvas.set_draw_color(Color::RGB(255, 255, 255));
    canvas.draw_rect(Rect::new(box_x, box_y, box_w, box_h as u32))?;

    let surface = font
        .render(text)
        .blended_wrapped(Color::RGB(255, 255, 255), box_w - 24)
        .map_err(|e| e.to_string())?;
    let texture = texture_creator
        .create_texture_from_surface(&surface)
        .map_err(|e| e.to_string())?;
    let q = texture.query();
    canvas.copy(&texture, None, Some(Rect::new(box_x + 12, box_y + 12, q.width, q.height)))?;

    let indicator = if is_last { "Fermer" } else { "Suite" };
    let ind_surf = font
        .render(indicator)
        .blended(Color::RGB(180, 180, 180))
        .map_err(|e| e.to_string())?;
    let ind_tex = texture_creator
        .create_texture_from_surface(&ind_surf)
        .map_err(|e| e.to_string())?;
    let (iw, ih) = (ind_tex.query().width, ind_tex.query().height);
    canvas.copy(
        &ind_tex, None,
        Some(Rect::new(box_x + box_w as i32 - iw as i32 - 12, box_y + box_h - ih as i32 - 8, iw, ih)),
    )?;

    Ok(())
}