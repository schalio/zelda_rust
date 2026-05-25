// src/inventory.rs

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::image::LoadTexture;

use crate::player::Player;

const WIN_W: u32 = 800;
const WIN_H: u32 = 600;

// Dimensions de la fenêtre d'inventaire
const INV_W: u32 = 400;
const INV_H: u32 = 300;
const INV_X: i32 = ((WIN_W - INV_W) / 2) as i32;
const INV_Y: i32 = ((WIN_H - INV_H) / 2) as i32;

const PAD: i32 = 16;
const ICON: u32 = 32; // taille des icônes

pub fn inventory_screen(
    canvas: &mut Canvas<Window>,
    event_pump: &mut sdl2::EventPump,
    player: &Player,
) -> Result<(), String> {
    let texture_creator = canvas.texture_creator();
    let objects_tex = texture_creator.load_texture("assets/sprites/objects.png")?;
    let hearts_tex  = texture_creator.load_texture("assets/sprites/hearts.png")?;
    let heart_pieces_tex = texture_creator.load_texture("assets/sprites/heart_pieces.png")?;

    // Taille d'un sprite dans objects.png (à adapter selon ton spritesheet)
    let obj_size: u32 = 16;

    'inv: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Ok(()),
                Event::KeyDown { keycode: Some(Keycode::I), .. }
                | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'inv,
                _ => {}
            }
        }

        // Fond semi-transparent sur tout l'écran
        canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 180));
        canvas.fill_rect(Rect::new(0, 0, WIN_W, WIN_H))?;

        // Panneau principal
        canvas.set_draw_color(Color::RGB(20, 20, 40));
        canvas.fill_rect(Rect::new(INV_X, INV_Y, INV_W, INV_H))?;
        canvas.set_draw_color(Color::RGB(180, 160, 80));
        canvas.draw_rect(Rect::new(INV_X, INV_Y, INV_W, INV_H))?;
        // Bordure double
        canvas.set_draw_color(Color::RGB(100, 80, 20));
        canvas.draw_rect(Rect::new(INV_X + 3, INV_Y + 3, INV_W - 6, INV_H - 6))?;

        let mid_x = INV_X + INV_W as i32 / 2;
        let section_h = INV_H as i32 * 2 / 3;

        // ── Séparateur vertical (haut) ──
        canvas.set_draw_color(Color::RGB(100, 80, 20));
        canvas.draw_line(
            sdl2::rect::Point::new(mid_x, INV_Y + PAD),
            sdl2::rect::Point::new(mid_x, INV_Y + section_h),
        )?;

        // ── Séparateur horizontal (bas) ──
        canvas.draw_line(
            sdl2::rect::Point::new(INV_X + PAD, INV_Y + section_h),
            sdl2::rect::Point::new(INV_X + INV_W as i32 - PAD, INV_Y + section_h),
        )?;

        // ════════════════════════════════
        // Section OBJETS (haut gauche)
        // ════════════════════════════════
        // Grille 4x2 de slots vides pour l'instant
        let slots_x = INV_X + PAD;
        let slots_y = INV_Y + PAD;
        let slot_size: i32 = 36;
        let slot_gap: i32 = 4;

        for row in 0..2i32 {
            for col in 0..4i32 {
                let sx = slots_x + col * (slot_size + slot_gap);
                let sy = slots_y + row * (slot_size + slot_gap);
                canvas.set_draw_color(Color::RGB(40, 40, 70));
                canvas.fill_rect(Rect::new(sx, sy, slot_size as u32, slot_size as u32))?;
                canvas.set_draw_color(Color::RGB(80, 80, 120));
                canvas.draw_rect(Rect::new(sx, sy, slot_size as u32, slot_size as u32))?;
            }
        }

        // ════════════════════════════════
        // Section QUARTS DE CŒUR (haut droite)
        // ════════════════════════════════
        // Affiche les cœurs complets + fraction en cours
        let hearts_x = mid_x + PAD;
        let hearts_y = INV_Y + PAD;

        let full_hearts  = player.max_hp / 2;   // 2 hp = 1 cœur
        let heart_sprite_size: u32 = 10;         // taille dans hearts.png
        let heart_draw: u32 = 20;
        let hgap: i32 = 4;
        let hearts_per_row = 5;

        for i in 0..full_hearts {
            let col = i % hearts_per_row;
            let row = i / hearts_per_row;
            let hx = hearts_x + col * (heart_draw as i32 + hgap);
            let hy = hearts_y + row * (heart_draw as i32 + hgap);

            // Cœur plein = colonne 0, rangée 0 dans hearts.png
            // (adapter selon ton spritesheet)
            let filled = player.hp >= (i + 1) * 2;
            let half   = !filled && player.hp >= i * 2 + 1;
            let src_col: i32 = if filled { 0 } else if half { 1 } else { 2 };

            let src = Rect::new(src_col * heart_sprite_size as i32, 0,
                                heart_sprite_size, heart_sprite_size);
            canvas.copy(&hearts_tex, Some(src),
                        Some(Rect::new(hx, hy, heart_draw, heart_draw)))?;
        }

        // Pièces de cœur collectées
        // heart_pieces.png : 4 colonnes (0/4, 1/4, 2/4, 3/4), 1 rangée
        let hp_sprite_w: u32 = 26;
        let hp_sprite_h: u32 = 26;
        let hp_draw: u32 = 48;

        let piece_col = player.heart_pieces.min(3) as i32;
        let src = Rect::new(
            piece_col * hp_sprite_w as i32,
            0,
            hp_sprite_w,
            hp_sprite_h,
        );

        // Positionner sous les cœurs de vie, centré dans la moitié droite
        let heart_rows = ((full_hearts + hearts_per_row - 1) / hearts_per_row).max(1);
        let hearts_block_h = heart_rows * heart_draw as i32 + (heart_rows - 1) * hgap;

        let top_right_x = mid_x;
        let top_right_y = INV_Y;
        let top_right_w = INV_W as i32 / 2;
        let top_right_h = section_h;

        let dest_x = top_right_x + (top_right_w - hp_draw as i32) / 2;
        let quarter_heart_offset_y = 48;
        let dest_y = hearts_y + hearts_block_h + quarter_heart_offset_y;

        // Optionnel : éviter que ça touche la séparation horizontale
        let max_y = top_right_y + top_right_h - PAD - hp_draw as i32;
        let dest_y = dest_y.min(max_y);

        canvas.copy(
            &heart_pieces_tex,
            Some(src),
            Some(Rect::new(dest_x, dest_y, hp_draw, hp_draw)),
        )?;

        // ════════════════════════════════
        // Section CLÉS (bas)
        // ════════════════════════════════
        let keys_y = INV_Y + section_h + PAD;

        let key_types: &[(u32, i32)] = &[
            (player.keys_silver, 10),
            (player.keys_gold, 11),
            (player.keys_boss, 12),
        ];

        let col_w = (INV_W as i32 - PAD * 2) / 3;
        for (idx, (count, sprite_col)) in key_types.iter().enumerate() {
            let col_x = INV_X + PAD + idx as i32 * col_w;
            let kx = col_x + (col_w - ICON as i32) / 2;

            // Icône clé — seulement si possédée
            if *count > 0 {
                let key_src = Rect::new(
                    sprite_col * obj_size as i32,
                    0,
                    obj_size,
                    obj_size,
                );
                canvas.copy(&objects_tex, Some(key_src),
                            Some(Rect::new(kx, keys_y, ICON, ICON)))?;
            } else {
                canvas.set_draw_color(Color::RGB(40, 40, 70));
                canvas.fill_rect(Rect::new(kx, keys_y, ICON, ICON))?;
                canvas.set_draw_color(Color::RGB(80, 80, 120));
                canvas.draw_rect(Rect::new(kx, keys_y, ICON, ICON))?;
            }
        }

        canvas.set_blend_mode(sdl2::render::BlendMode::None);
        canvas.present();
    }

    Ok(())
}