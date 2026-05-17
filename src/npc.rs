use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;
use sdl2::pixels::Color;

use crate::camera::Camera;
use crate::tilemap::TILE_DRAW_SIZE;
use crate::player::Direction;

/// Dimensions des sprites PNJ dans le spritesheet (pixels source)
const SPRITE_W: u32 = 16;
const SPRITE_H: u32 = 26;

/// Taille affichée (agrandie comme les tuiles)
const DRAW_W: u32 = (TILE_DRAW_SIZE as f64 * 0.5) as u32;
const DRAW_H: u32 = (DRAW_W as f64 * 26.0 / 16.0) as u32;  // ratio 26/16

pub const NPC_PATROL_SPEED: f32 = 48.0; // pixels par seconde (≈ 1.5 tiles/s)
const NPC_HALF: f32 = 7.0; // demi-côté hitbox NPC pour les collisions tilemap


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcKind {
    Villager,
    Animal,
    Guy,
    Girl,
}

#[derive(Debug, Clone)]
pub struct Npc {
    pub x: f32,
    pub y: f32,
    pub kind: NpcKind,
    pub solid: bool,
    pub dialogue: String,
    pub anim_timer: f32,
    pub anim_frame: u32,
    // --- Patrouille ---
    // pub patrol_target: Option<(f32, f32)>,  // destination courante (absolue)
    // pub patrol_origin: Option<(f32, f32)>,  // position de départ (absolue)
    pub waypoints:     Option<Vec<(f32, f32)>>,
    pub waypoint_idx:  usize,
    pub patrol_going: bool,                 // true = vers target, false = retour
    pub direction: u32,                     // 0=bas 1=haut 2=gauche 3=droite
}

/// Retourne la ligne du spritesheet pour ce type de PNJ.
/// Ligne 0=Villager, 1=Animal, 2=Guy, 3=Girl
fn sprite_row(kind: NpcKind) -> i32 {
    match kind {
        NpcKind::Villager => 0,
        NpcKind::Animal   => 1,
        NpcKind::Guy      => 2,
        NpcKind::Girl     => 3,
    }
}


pub fn update_npcs(
    npcs: &mut Vec<Npc>,
    dt: f32,
    player_x: f32,
    player_y: f32,
    tilemap: &crate::tilemap::Tilemap,
    tile_table: &crate::tile_properties::TileTable,
) {
    for npc in npcs.iter_mut() {
        npc.anim_timer += dt;

        let Some(ref waypoints) = npc.waypoints else { continue; };

        if waypoints.len() < 2 { continue; }

        let (tx, ty) = waypoints[npc.waypoint_idx];
        let dx = tx - npc.x;
        let dy = ty - npc.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 2.0 {
            // Arrivé au waypoint → waypoint suivant (ping-pong)
            npc.x = tx;
            npc.y = ty;

            if npc.patrol_going {
                if npc.waypoint_idx + 1 >= waypoints.len() {
                    npc.patrol_going = false;
                    npc.waypoint_idx -= 1;
                } else {
                    npc.waypoint_idx += 1;
                }
            } else {
                if npc.waypoint_idx == 0 {
                    npc.patrol_going = true;
                    npc.waypoint_idx += 1;
                } else {
                    npc.waypoint_idx -= 1;
                }
            }
        } else {
            let nx = dx / dist;
            let ny = dy / dist;

            let new_x = npc.x + nx * NPC_PATROL_SPEED * dt;
            if !npc_collides_with_tilemap(new_x, npc.y, tilemap, tile_table) {
                npc.x = new_x;
            }
            let new_y = npc.y + ny * NPC_PATROL_SPEED * dt;
            if !npc_collides_with_tilemap(npc.x, new_y, tilemap, tile_table) {
                npc.y = new_y;
            }

            npc.direction = direction_from_vector(nx, ny);

            if npc.solid {
                push_npc_away_from_player(npc, player_x, player_y);
            }
        }
    }
}

/// Repousse le NPC si sa hitbox chevauche celle du joueur.
/// On utilise une hitbox carrée simple : demi-côté de 8px pour chacun.
fn push_npc_away_from_player(npc: &mut Npc, player_x: f32, player_y: f32) {
    use crate::tilemap::TILE_DRAW_SIZE;

    // Demi-dimensions joueur (identiques à HITBOX_HALF_W/H dans player.rs)
    let player_half_w = TILE_DRAW_SIZE as f32 * 0.30;
    let player_half_h = TILE_DRAW_SIZE as f32 * 0.30;

    // Centre NPC (npc.x/y = coin haut-gauche)
    let npc_draw_w = TILE_DRAW_SIZE as f32 * 0.5;
    let npc_draw_h = npc_draw_w * 26.0 / 16.0;
    let npc_cx = npc.x + npc_draw_w * 0.5;
    let npc_cy = npc.y + npc_draw_h * 0.5;
    let npc_half_w = npc_draw_w * 0.5;
    let npc_half_h = npc_draw_h * 0.5;

    let sum_w = player_half_w + npc_half_w;
    let sum_h = player_half_h + npc_half_h;

    let dx = npc_cx - player_x;
    let dy = npc_cy - player_y;

    let overlap_x = sum_w - dx.abs();
    let overlap_y = sum_h - dy.abs();

    if overlap_x > 0.0 && overlap_y > 0.0 {
        // Repousser sur l'axe avec le moins de chevauchement
        if overlap_x < overlap_y {
            let push = if dx >= 0.0 { overlap_x } else { -overlap_x };
            npc.x += push;  // on déplace le coin, pas le centre
        } else {
            let push = if dy >= 0.0 { overlap_y } else { -overlap_y };
            npc.y += push;
        }
    }
}

/// Teste si la hitbox du NPC (carré centré) touche une tuile solide.
/// On vérifie les 4 coins — même technique que le joueur.
fn npc_collides_with_tilemap(
    cx: f32,
    cy: f32,
    tilemap: &crate::tilemap::Tilemap,
    table: &crate::tile_properties::TileTable,
) -> bool {
    let h = NPC_HALF - 1.0; // légère marge pour éviter le blocage sur les bords
    tilemap.is_solid_at(cx - h, cy - h, table)
        || tilemap.is_solid_at(cx + h, cy - h, table)
        || tilemap.is_solid_at(cx - h, cy + h, table)
        || tilemap.is_solid_at(cx + h, cy + h, table)
}


/// Convertit un vecteur normalisé en index de direction sprite.
/// Spritesheet : 0=bas  1=haut  2=gauche  3=droite
fn direction_from_vector(nx: f32, ny: f32) -> u32 {
    if ny.abs() >= nx.abs() {
        if ny > 0.0 { 0 } else { 1 } // bas ou haut
    } else {
        if nx < 0.0 { 2 } else { 3 } // gauche ou droite
    }
}


pub fn render_npcs(
    canvas: &mut Canvas<Window>,
    texture: &Texture,
    camera: &Camera,
    npcs: &[Npc],
) -> Result<(), String> {
    for npc in npcs {

        // APRÈS (direction + bobbing)
        let bob_y = (npc.anim_timer * std::f32::consts::PI * 2.0 / 0.5).sin() * 2.0;

        // Colonne = type de PNJ (Villager=0, Animal=1, …)
        let src_y = sprite_row(npc.kind) * SPRITE_H as i32;
        // Ligne = direction (0=bas, 1=haut, 2=gauche, 3=droite)
        let src_x = npc.direction as i32 * SPRITE_W as i32;

        let src = Rect::new(src_x, src_y, SPRITE_W, SPRITE_H);

        let dst = Rect::new(
            (npc.x - camera.x) as i32,
            (npc.y - camera.y) as i32 + bob_y as i32,
            DRAW_W,
            DRAW_H,
        );

        canvas.copy(texture, src, dst)?;
    }

    Ok(())
}

pub fn collides_with_npc_rect(
    player_cx: f32,
    player_cy: f32,
    half_w: f32,
    half_h: f32,
    npcs: &[Npc],
) -> bool {
    use crate::tilemap::TILE_DRAW_SIZE;

    for npc in npcs.iter().filter(|n| n.solid) {
        let npc_draw_w = TILE_DRAW_SIZE as f32 * 0.5;
        let npc_draw_h = npc_draw_w * 26.0 / 16.0;

        // npc.x/npc.y = coin haut-gauche, déjà en pixels-monde
        let npc_cx = npc.x + npc_draw_w * 0.5;
        let npc_cy = npc.y + npc_draw_h * 0.5;

        // sum_w/sum_h doivent couvrir dx≈80, dy≈4
        // on prend la moitié du sprite complet de chaque côté
        let npc_half_w = npc_draw_w * 0.5;
        let npc_half_h = npc_draw_h * 0.5;

        let dx = (player_cx - npc_cx).abs();
        let dy = (player_cy - npc_cy).abs();
        let sum_w = half_w + npc_half_w;
        let sum_h = half_h + npc_half_h;

        if dx < sum_w && dy < sum_h {
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

pub fn render_npc_hitboxes(
    npcs: &[Npc],
    canvas: &mut Canvas<Window>,
    camera: &Camera,
) -> Result<(), String> {

    let npc_draw_w = TILE_DRAW_SIZE as f32 * 0.5;
    let npc_draw_h = npc_draw_w * 26.0 / 16.0;

    for npc in npcs.iter().filter(|n| n.solid) {
        let npc_world_x = npc.x;
        let npc_world_y = npc.y;
        let npc_cx = npc_world_x + npc_draw_w * 0.5;
        let npc_cy = npc_world_y + npc_draw_h * 0.5;
        let npc_half_w = npc_draw_w * 0.30;
        let npc_half_h = npc_draw_h * 0.30;

        let (sx, sy) = camera.world_to_screen(npc_cx - npc_half_w, npc_cy - npc_half_h);
        canvas.set_draw_color(Color::RGB(255, 0, 255)); // magenta
        canvas.draw_rect(Rect::new(
            sx, sy,
            (npc_half_w * 2.0) as u32,
            (npc_half_h * 2.0) as u32,
        ))?;
    }
    Ok(())
}