use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::camera::Camera;
use crate::tilemap::TILE_DRAW_SIZE;
use crate::player::Direction;

/// Dimensions des sprites PNJ dans le spritesheet (pixels source)
const SPRITE_W: u32 = 16;
const SPRITE_H: u32 = 26;

/// Taille affichée (agrandie comme les tuiles)
const DRAW_W: u32 = (TILE_DRAW_SIZE as f64 * 0.5) as u32;
const DRAW_H: u32 = (DRAW_W as f64 * 26.0 / 16.0) as u32;  // ratio 26/16

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcKind {
    Villager,
    Animal,
}

#[derive(Debug, Clone)]
pub struct Npc {
    pub x: f32,
    pub y: f32,
    pub kind: NpcKind,
    pub solid: bool,
    pub dialogue: String,
}

fn sprite_col(kind: NpcKind) -> i32 {
    match kind {
        NpcKind::Villager => 0,
        NpcKind::Animal => 1,
    }
}

pub fn render_npcs(
    canvas: &mut Canvas<Window>,
    texture: &Texture,
    camera: &Camera,
    npcs: &[Npc],
) -> Result<(), String> {
    for npc in npcs {
        let src_x = sprite_col(npc.kind) * SPRITE_W as i32;
        let src = Rect::new(src_x, 0, SPRITE_W, SPRITE_H);
        let dst = Rect::new(
            (npc.x - camera.x) as i32,
            (npc.y - camera.y) as i32,
            DRAW_W,
            DRAW_H,
        );

        canvas.copy(texture, src, dst)?;
    }

    Ok(())
}

pub fn collides_with_npc_rect(
    next_x: f32,
    next_y: f32,
    player_w: f32,
    player_h: f32,
    npcs: &[Npc],
) -> bool {
    for npc in npcs.iter().filter(|n| n.solid) {
        // Hitbox du PNJ : un peu plus petite que le sprite pour le feeling
        let npc_w = TILE_DRAW_SIZE as f32 * 0.75;
        let npc_h = (TILE_DRAW_SIZE as f32 * 26.0 / 16.0) * 0.75;

        let overlap = next_x < npc.x + npc_w
            && next_x + player_w > npc.x
            && next_y < npc.y + npc_h
            && next_y + player_h > npc.y;

        if overlap {
            return true;
        }
    }

    false
}

pub fn find_npc_in_front(
    player_x: f32,
    player_y: f32,
    direction: Direction,
    npcs: &[Npc],
) -> Option<usize> {
    let reach = TILE_DRAW_SIZE as f32 * 0.9;
    let zone_w = TILE_DRAW_SIZE as f32 * 0.8;
    let zone_h = TILE_DRAW_SIZE as f32 * 0.8;

    let (zx, zy) = match direction {
        Direction::Up    => (player_x - zone_w * 0.5, player_y - reach - zone_h * 0.5),
        Direction::Down  => (player_x - zone_w * 0.5, player_y + reach - zone_h * 0.5),
        Direction::Left  => (player_x - reach - zone_w * 0.5, player_y - zone_h * 0.5),
        Direction::Right => (player_x + reach - zone_w * 0.5, player_y - zone_h * 0.5),
    };

    for (i, npc) in npcs.iter().enumerate() {
        let npc_w = DRAW_W as f32 * 0.75;
        let npc_h = DRAW_H as f32 * 0.75;

        let overlap = zx < npc.x + npc_w
            && zx + zone_w > npc.x
            && zy < npc.y + npc_h
            && zy + zone_h > npc.y;

        if overlap {
            return Some(i);
        }
    }

    None
}