// src/objects.rs

use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::camera::Camera;
use crate::map_loader::{MapObject, ObjectKind, RubyKind};
use crate::tilemap::TILE_DRAW_SIZE;
use crate::player::HITBOX_HALF;

const CHEST_HALF: f32 = TILE_DRAW_SIZE as f32 * 0.3;
const SPRITE_SIZE: u32 = 16;   // taille dans le spritesheet
const DRAW_SIZE: u32   = TILE_DRAW_SIZE / 2; // taille affichée (48px)

/// Retourne la colonne dans le spritesheet selon le type d'objet
fn sprite_col(kind: &ObjectKind) -> Option<u32> {
    match kind {
        ObjectKind::Heart                   => Some(0),
        ObjectKind::Ruby(RubyKind::Green)   => Some(1),
        ObjectKind::Key                     => Some(2),
        ObjectKind::Chest { .. }            => Some(3),
        ObjectKind::HeartPiece { .. }       => Some(5),
        ObjectKind::Transition { .. }       => None,
        ObjectKind::Bush                    => Some(6),
        ObjectKind::Ruby(RubyKind::Blue)    => Some(7),
        ObjectKind::Ruby(RubyKind::Red)     => Some(8),
    }
}

pub fn render_objects(
    objects: &[MapObject],
    canvas: &mut Canvas<Window>,
    sheet: &Texture,
    camera: &Camera,
) -> Result<(), String> {
    for obj in objects {
        // Les coffres ouverts (collected=true) affichent le sprite ouvert col 4
        // Les autres objets collectés sont déjà retirés de la liste
        if obj.collected && !matches!(obj.kind, ObjectKind::Chest { .. }) {
            continue;  // ← HeartPiece collecté = invisible
        }

        let col = if matches!(obj.kind, ObjectKind::Chest { .. }) && obj.collected {
            4  // coffre ouvert
        } else {
            match sprite_col(&obj.kind) {
                Some(c) => c,
                None    => continue,
            }
        };

        let src = Rect::new(
            (col * SPRITE_SIZE) as i32,
            0,
            SPRITE_SIZE,
            SPRITE_SIZE,
        );

        let draw_size = match obj.kind {
            ObjectKind::Bush => crate::tilemap::TILE_DRAW_SIZE,
            _ => DRAW_SIZE, // ta taille actuelle pour les petits objets
        } as u32;

        let screen_x = (obj.x - camera.x) as i32;
        let screen_y = (obj.y - camera.y) as i32;

        let dst = Rect::new(screen_x, screen_y, draw_size, draw_size);
        canvas.copy(sheet, src, dst)?;
    }
    Ok(())
}

pub fn resolve_chest_collision(player: &mut crate::player::Player, objects: &[MapObject]) {
    for obj in objects {
        // Seuls les coffres fermés bloquent
        if !matches!(obj.kind, ObjectKind::Chest { .. }) { continue; }
        // if obj.collected { continue; }  // coffre ouvert = traversable

        let overlap_x = (player.x - obj.x).abs() < HITBOX_HALF + CHEST_HALF;
        let overlap_y = (player.y - obj.y).abs() < HITBOX_HALF + CHEST_HALF;

        if overlap_x && overlap_y {
            // Repousse le joueur hors du coffre
            let dx = player.x - obj.x;
            let dy = player.y - obj.y;

            let pen_x = (HITBOX_HALF + CHEST_HALF) - dx.abs();
            let pen_y = (HITBOX_HALF + CHEST_HALF) - dy.abs();

            if pen_x < pen_y {
                player.x += pen_x * dx.signum();
            } else {
                player.y += pen_y * dy.signum();
            }
        }
    }
}