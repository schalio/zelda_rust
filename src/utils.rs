// src/utils.rs

use crate::enemy::Enemy;
use crate::player;
use crate::tile_properties::TileTable;
use crate::tilemap::{Tilemap, TILE_DRAW_SIZE};

pub fn split_dialogue(text: &str) -> Vec<String> {
    let normalized = text.replace("\\n", "\n");
    normalized.split("\n\n")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn is_in_front_of_player(
    px: f32, py: f32,
    dir: player::Direction,
    obj_x: f32, obj_y: f32,
) -> bool {
    let reach = TILE_DRAW_SIZE as f32 * 1.2;
    let ox = obj_x + TILE_DRAW_SIZE as f32 * 0.5;
    let oy = obj_y + TILE_DRAW_SIZE as f32 * 0.5;
    let (dx, dy) = (ox - px, oy - py);
    match dir {
        player::Direction::Up    => dy < 0.0 && dy.abs() < reach && dx.abs() < reach,
        player::Direction::Down  => dy > 0.0 && dy.abs() < reach && dx.abs() < reach,
        player::Direction::Left  => dx < 0.0 && dx.abs() < reach && dy.abs() < reach,
        player::Direction::Right => dx > 0.0 && dx.abs() < reach && dy.abs() < reach,
    }
}

pub fn separate_enemies(enemies: &mut [Enemy], tilemap: &Tilemap, table: &TileTable) {
    for i in 0..enemies.len() {
        for j in (i + 1)..enemies.len() {
            if !enemies[i].is_alive || !enemies[j].is_alive { continue; }

            let dx = enemies[j].x - enemies[i].x;
            let dy = enemies[j].y - enemies[i].y;
            let dist_sq = dx * dx + dy * dy;

            let min_dist = enemies[i].separation_radius() + enemies[j].separation_radius();

            if dist_sq < min_dist * min_dist {
                if dist_sq < 0.0001 {
                    enemies[i].push_by(-1.0, 0.0, tilemap, table);
                    enemies[j].push_by( 1.0, 0.0, tilemap, table);
                    continue;
                }
                let dist   = dist_sq.sqrt();
                let overlap = min_dist - dist;
                let nx = dx / dist;
                let ny = dy / dist;
                enemies[i].push_by(-nx * overlap * 0.5, -ny * overlap * 0.5, tilemap, table);
                enemies[j].push_by( nx * overlap * 0.5,  ny * overlap * 0.5, tilemap, table);
            }
        }
    }
}