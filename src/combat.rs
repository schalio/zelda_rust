// src/combat.rs
// Résout les interactions de combat entre le joueur et les ennemis.

use crate::audio::AudioManager;
use crate::player::{Player, DeathState};
use crate::enemy::Enemy;
use crate::map_loader::{MapObject, ObjectKind, LootKind};

const PICKUP_RANGE: f32 = crate::tilemap::TILE_DRAW_SIZE as f32 * 0.6;

/// Teste si deux rectangles AABB se chevauchent.
/// Chaque rectangle est défini par son centre (cx, cy) et ses demi-dimensions.
pub fn aabb_overlap(
    cx1: f32, cy1: f32, hw1: f32, hh1: f32,
    cx2: f32, cy2: f32, hw2: f32, hh2: f32,
) -> bool {
    (cx1 - cx2).abs() < hw1 + hw2
        && (cy1 - cy2).abs() < hh1 + hh2
}

/// Résout les dégâts des ennemis sur le joueur (contact damage).
/// Appelée une fois par frame après update() de tous les ennemis.
pub fn resolve_enemy_contact(player: &mut Player, enemies: &mut [Enemy], audio: &AudioManager) {
    if !player.is_alive() { return; }

    // ← supprimez la ligne : let was_alive = player.is_alive();

    for enemy in enemies.iter() {
        if !enemy.is_alive { continue; }

        let hits = aabb_overlap(
            player.x, player.y,
            crate::player::HITBOX_HALF,
            crate::player::HITBOX_HALF,
            enemy.x, enemy.y,
            crate::enemy::ENEMY_HITBOX_HALF,
            crate::enemy::ENEMY_HITBOX_HALF,
        );

        if hits {
            let state_before  = player.death_state;
            let was_invincible = player.is_invincible;

            player.take_hit(enemy.x, enemy.y, enemy.contact_damage());

            let state_after = player.death_state;

            // println!("→ state_before={:?} state_after={:?} was_invincible={}", state_before, state_after, was_invincible);

            if state_before == DeathState::Alive && state_after == DeathState::Spinning {
            //    println!("→ APPEL play_player_death");
                audio.play_player_death();
            } else if !was_invincible && state_after == DeathState::Alive {
                audio.play_hit_player();
            }
        }
    }
}

/// Résout les dégâts de l'attaque du joueur sur les ennemis.
/// Appelée uniquement pendant les frames où l'attaque est active.
pub fn resolve_player_attack(player: &Player, enemies: &mut [Enemy], audio: &AudioManager) {
    if !player.is_alive() { return; }

    // Si le joueur n'attaque pas, rien à faire
    let Some((atk_x, atk_y, atk_hw, atk_hh)) = player.attack_hitbox() else {
        return;
    };

    for enemy in enemies.iter_mut() {
        if !enemy.is_alive { continue; }

        let hits = aabb_overlap(
            atk_x, atk_y, atk_hw, atk_hh,
            enemy.x, enemy.y,
            crate::enemy::ENEMY_HITBOX_HALF,
            crate::enemy::ENEMY_HITBOX_HALF,
        );

        if hits {
            let was_alive = enemy.is_alive;
            enemy.take_hit(player.x, player.y, player.attack_damage());

            if !enemy.is_alive && was_alive {
                audio.play_enemy_death();
            } else if enemy.is_alive {
                audio.play_hit_player();
            }

        }
    }
}

pub fn resolve_object_contact(player: &mut Player, objects: &mut Vec<MapObject>, audio: &AudioManager) {
    if !player.is_alive() { return; }

    let mut to_spawn: Vec<MapObject> = Vec::new();

    for obj in objects.iter_mut() {
        if obj.collected { continue; }

        let dx = player.x - obj.x;
        let dy = player.y - obj.y;
        if (dx * dx + dy * dy).sqrt() > PICKUP_RANGE { continue; }

        match obj.kind {
            ObjectKind::Heart                  => { player.hp = (player.hp + 2).min(player.max_hp); obj.collected = true; audio.play_pickup_heart(); }
            ObjectKind::Ruby                   => { player.rubies += 1; obj.collected = true; audio.play_pickup_ruby(); }
            ObjectKind::Key                    => { player.keys += 1;   obj.collected = true; audio.play_pickup_ruby();}
            ObjectKind::Chest { contains }     => {
                obj.collected = true;
                audio.play_chest_open();
                to_spawn.push(MapObject {
                    x: obj.x,
                    y: obj.y - crate::tilemap::TILE_DRAW_SIZE as f32 * 0.5,
                    kind: match contains {
                        LootKind::Heart => ObjectKind::Heart,
                        LootKind::Ruby  => ObjectKind::Ruby,
                        LootKind::Key   => ObjectKind::Key,
                    },
                    collected: false,
                });
            }
        }
    }

    objects.extend(to_spawn);
    objects.retain(|o| !o.collected || matches!(o.kind, ObjectKind::Chest { .. }));
}