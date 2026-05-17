// src/enemy.rs

// use sdl2::keyboard::Scancode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::camera::Camera;
use crate::enemy_type::{EnemyKind, EnemyStats};
use crate::tilemap::{Tilemap, TILE_DRAW_SIZE};
use crate::tile_properties::TileTable;

const SPRITE_W: u32 = 22;
const SPRITE_H: u32 = 28;
const ANIM_FRAMES: u32 = 4;
const FRAME_DURATION: f32 = 0.18; // un peu plus lent que le joueur
// pub const ENEMY_RADIUS: f32 = TILE_DRAW_SIZE as f32 * 0.28;
pub const ENEMY_HITBOX_HALF: f32 = TILE_DRAW_SIZE as f32 * 0.28;

const ENEMY_KNOCKBACK_SPEED: f32    = 200.0;
const ENEMY_KNOCKBACK_DURATION: f32 = 0.12;
const ENEMY_INVINCIBILITY: f32      = 0.3;

const VANISH_FRAMES: u32    = 2;
const VANISH_DURATION: f32  = 0.3;   // durée totale (identique à death_timer)
const VANISH_SPRITE_W: u32  = 16;
const VANISH_SPRITE_H: u32  = 16;
const VANISH_DRAW_SIZE: u32 = TILE_DRAW_SIZE;
const VANISH_FRAME_DT: f32  = VANISH_DURATION / VANISH_FRAMES as f32;

/// Les deux états possibles de l'IA d'un ennemi.
#[derive(Clone, Copy, PartialEq)]
enum AiState {
    /// L'ennemi patrouille entre son point d'origine et un point cible.
    Patrolling,
    /// L'ennemi a détecté le joueur et le poursuit.
    Chasing,
}

pub struct Enemy {
    /// Position courante dans le monde (centre du sprite)
    pub x: f32,
    pub y: f32,

    /// Point d'origine de la patrouille (position de spawn)
    origin_x: f32,
    origin_y: f32,

    /// Point cible actuel de la patrouille
    patrol_target_x: f32,
    patrol_target_y: f32,

    /// Vrai si l'ennemi va vers la cible (false = il revient à l'origine)
    patrol_going: bool,

    pub hp: i32,
    pub is_alive: bool,
    is_invincible: bool,
    invincibility_timer: f32,
    knockback_x: f32,
    knockback_y: f32,
    knockback_timer: f32,
    pub death_timer: f32,      // pour l'animation de mort
    vanish_frame: u32,

    kind: EnemyKind,
    stats: EnemyStats,

    ai_state: AiState,

    // Animation
    anim_frame: u32,
    anim_timer: f32,
    /// Direction courante pour le spritesheet (0=bas,1=gauche,2=droite,3=haut)
    sprite_row: u32,
}

impl Enemy {
    /// Crée un ennemi à la position donnée.
    /// La direction de patrouille initiale est horizontale (vers la droite).
    pub fn new(x: f32, y: f32, kind: EnemyKind) -> Self {
        let stats = kind.stats();
        let patrol_range = stats.patrol_range;
        let hp = stats.max_hp;
        Enemy {
            x,
            y,
            origin_x: x,
            origin_y: y,
            patrol_target_x: x + patrol_range,
            patrol_target_y: y,
            patrol_going: true,
            hp,
            is_alive: true,
            kind,
            stats,
            ai_state: AiState::Patrolling,
            anim_frame: 0,
            anim_timer: 0.0,
            sprite_row: 0, // face au bas par défaut
            is_invincible: false,
            invincibility_timer: 0.0,
            knockback_x: 0.0,
            knockback_y: 0.0,
            knockback_timer: 0.0,
            death_timer: 0.0,
            vanish_frame: 0,
        }
    }

    /// Rayon utilisé pour éviter que les ennemis ne se superposent.
    pub fn separation_radius(&self) -> f32 {
        TILE_DRAW_SIZE as f32 * 0.30
    }

    /// Déplace l'ennemi d'un petit vecteur de correction.
    pub fn push_by(&mut self, dx: f32, dy: f32, tilemap: &Tilemap, table: &TileTable) {
        if !self.is_alive {
            return;
        }

        let next_x = self.x + dx;
        let next_y = self.y + dy;
        let half = self.separation_radius();

        let blocked_x = tilemap.is_solid_at(next_x - half, self.y - half, table)
            || tilemap.is_solid_at(next_x + half, self.y - half, table)
            || tilemap.is_solid_at(next_x - half, self.y + half, table)
            || tilemap.is_solid_at(next_x + half, self.y + half, table);

        if !blocked_x {
            self.x = next_x;
        }

        let blocked_y = tilemap.is_solid_at(self.x - half, next_y - half, table)
            || tilemap.is_solid_at(self.x + half, next_y - half, table)
            || tilemap.is_solid_at(self.x - half, next_y + half, table)
            || tilemap.is_solid_at(self.x + half, next_y + half, table);

        if !blocked_y {
            self.y = next_y;
        }
    }

    /// Mise à jour de l'IA et du mouvement.
    pub fn update(
        &mut self,
        dt: f32,
        player_x: f32,
        player_y: f32,
        tilemap: &Tilemap,
        table: &TileTable,
    ) {
        // Bloc mort — remplace l'ancien
        if !self.is_alive {
            if self.death_timer > 0.0 {
                self.death_timer -= dt;

                // Calcul de la frame vanish courante
                let elapsed = VANISH_DURATION - self.death_timer;
                self.vanish_frame =
                    ((elapsed / VANISH_FRAME_DT) as u32).min(VANISH_FRAMES - 1);
            }
            return;
        }

        if self.is_invincible {
            self.invincibility_timer -= dt;
            if self.invincibility_timer <= 0.0 {
                self.is_invincible = false;
            }
        }

        // Knockback prioritaire sur l'IA
        if self.knockback_timer > 0.0 {
            self.knockback_timer -= dt;

            let next_x = self.x + self.knockback_x * dt;
            let half = ENEMY_HITBOX_HALF;
            if !tilemap.is_solid_at(next_x - half, self.y, table)
                && !tilemap.is_solid_at(next_x + half, self.y, table) {
                self.x = next_x;
            }

            let next_y = self.y + self.knockback_y * dt;
            if !tilemap.is_solid_at(self.x, next_y - half, table)
                && !tilemap.is_solid_at(self.x, next_y + half, table) {
                self.y = next_y;
            }
            return; // Pas d'IA pendant le knockback
        }

        // --- Calcul de la distance au joueur ---
        let dist_to_player = distance(self.x, self.y, player_x, player_y);

        // --- Transition d'état IA ---
        self.ai_state = if dist_to_player <= self.stats.detection_range {
            AiState::Chasing
        } else {
            AiState::Patrolling
        };

        // --- Mouvement selon l'état ---
        let (dx, dy) = match self.ai_state {
            AiState::Chasing => {
                // Vecteur normalisé vers le joueur
                let dir = normalize(player_x - self.x, player_y - self.y);
                dir
            }
            AiState::Patrolling => {
                // Cible courante : aller ou retour
                let (tx, ty) = if self.patrol_going {
                    (self.patrol_target_x, self.patrol_target_y)
                } else {
                    (self.origin_x, self.origin_y)
                };

                let dist_to_target = distance(self.x, self.y, tx, ty);

                // Arrivé près de la cible → inverser la direction
                if dist_to_target < 4.0 {
                    self.patrol_going = !self.patrol_going;
                }

                normalize(tx - self.x, ty - self.y)
            }
        };

        // --- Mise à jour de la direction sprite ---
        // On choisit la direction dominante pour le spritesheet
        if dy.abs() > dx.abs() {
            self.sprite_row = if dy < 0.0 { 3 } else { 0 }; // haut ou bas
        } else if dx != 0.0 {
            self.sprite_row = if dx < 0.0 { 1 } else { 2 }; // gauche ou droite
        }

        // --- Collision avec les tuiles ---
        let move_x = dx * self.stats.speed * dt;
        let move_y = dy * self.stats.speed * dt;
        let half = TILE_DRAW_SIZE as f32 * 0.28;

        // Axe X
        let next_x = self.x + move_x;
        let blocked_x = tilemap.is_solid_at(next_x - half, self.y, table)
            || tilemap.is_solid_at(next_x + half, self.y, table);
        if !blocked_x { self.x = next_x; } else {
            // En patrouille : inverser si on touche un mur
            if self.ai_state == AiState::Patrolling {
                self.patrol_going = !self.patrol_going;
            }
        }

        // Axe Y
        let next_y = self.y + move_y;
        let blocked_y = tilemap.is_solid_at(self.x, next_y - half, table)
            || tilemap.is_solid_at(self.x, next_y + half, table);
        if !blocked_y { self.y = next_y; } else {
            if self.ai_state == AiState::Patrolling {
                self.patrol_going = !self.patrol_going;
            }
        }

        // --- Animation ---
        let is_moving = dx.abs() > 0.01 || dy.abs() > 0.01;
        if is_moving {
            self.anim_timer += dt;
            if self.anim_timer >= FRAME_DURATION {
                self.anim_timer -= FRAME_DURATION;
                self.anim_frame = (self.anim_frame + 1) % ANIM_FRAMES;
            }
        } else {
            self.anim_frame = 0;
            self.anim_timer = 0.0;
        }
    }

    /// Inflige des dégâts à l'ennemi.
    pub fn take_damage(&mut self, amount: i32) {
        self.hp -= amount;
        if self.hp <= 0 {
            self.hp = 0;
            self.is_alive = false;
        }
    }

    /// Rendu de l'ennemi avec son spritesheet.
    pub fn render(
        &self,
        canvas: &mut Canvas<Window>,
        spritesheet: &Texture,
        vanish_sheet: &Texture,
        camera: &Camera,
    ) -> Result<(), String> {
        // --- Animation de mort ---
        if !self.is_alive {
            if self.death_timer > 0.0 {
                let src = Rect::new(
                    (self.vanish_frame * VANISH_SPRITE_W) as i32,
                    0,
                    VANISH_SPRITE_W,
                    VANISH_SPRITE_H,
                );

                let half = (VANISH_DRAW_SIZE / 2) as f32;
                let (sx, sy) = camera.world_to_screen(self.x - half, self.y - half);
                let dst = Rect::new(sx, sy, VANISH_DRAW_SIZE, VANISH_DRAW_SIZE);

                canvas.copy(vanish_sheet, Some(src), Some(dst))?;
            }
            return Ok(());
        }

        // Ligne du spritesheet = type d'ennemi × 4 directions + direction courante
        // Organisation : toutes les anims du Slime, puis Goblin, puis Knight
        let sheet_row = self.stats.sprite_row * 4 + self.sprite_row;

        let src_rect = Rect::new(
            (self.anim_frame * SPRITE_W) as i32,
            (sheet_row * SPRITE_H) as i32,
            SPRITE_W,
            SPRITE_H,
        );

        let half = (TILE_DRAW_SIZE / 2) as f32;
        let (sx, sy) = camera.world_to_screen(self.x - half, self.y - half);
        canvas.copy(spritesheet, Some(src_rect), Rect::new(sx, sy, TILE_DRAW_SIZE, TILE_DRAW_SIZE))?;

        // Barre de vie au-dessus du sprite
        self.render_health_bar(canvas, camera)?;

        Ok(())
    }

    /// Dessine une petite barre de vie colorée au-dessus du sprite.
    fn render_health_bar(
        &self,
        canvas: &mut Canvas<Window>,
        camera: &Camera,
    ) -> Result<(), String> {
        let bar_w = TILE_DRAW_SIZE;
        let bar_h = 4u32;
        let half = (TILE_DRAW_SIZE / 2) as f32;

        let (sx, sy) = camera.world_to_screen(self.x - half, self.y - half - 8.0);

        // Fond gris
        canvas.set_draw_color(Color::RGB(60, 60, 60));
        canvas.fill_rect(Rect::new(sx, sy, bar_w, bar_h))?;

        // Barre verte proportionnelle aux PV restants
        let fill_w = (bar_w as f32 * self.hp as f32 / self.stats.max_hp as f32) as u32;
        canvas.set_draw_color(Color::RGB(80, 200, 80));
        canvas.fill_rect(Rect::new(sx, sy, fill_w, bar_h))?;

        Ok(())
    }

    /// Rendu debug : affiche le rayon de détection et la hitbox.
    pub fn render_debug(&self, canvas: &mut Canvas<Window>, camera: &Camera) -> Result<(), String> {
        if !self.is_alive { return Ok(()); }
        let half_hb = TILE_DRAW_SIZE as f32 * 0.28;
        let (sx, sy) = camera.world_to_screen(self.x - half_hb, self.y - half_hb);
        canvas.set_draw_color(Color::RGBA(255, 80, 80, 160));
        canvas.draw_rect(Rect::new(sx, sy, (half_hb * 2.0) as u32, (half_hb * 2.0) as u32))?;
        Ok(())
    }

    pub fn contact_damage(&self) -> i32 {
        match self.kind {
            EnemyKind::Slime => 1,
            EnemyKind::Goblin => 1,
            EnemyKind::Knight => 2,
        }
    }

    pub fn collides_with_player(&self, player_x: f32, player_y: f32, player_half: f32) -> bool {
        if !self.is_alive {
            return false;
        }

        let enemy_half = TILE_DRAW_SIZE as f32 * 0.28;

        let left_a   = self.x - enemy_half;
        let right_a  = self.x + enemy_half;
        let top_a    = self.y - enemy_half;
        let bottom_a = self.y + enemy_half;

        let left_b   = player_x - player_half;
        let right_b  = player_x + player_half;
        let top_b    = player_y - player_half;
        let bottom_b = player_y + player_half;

        !(right_a < left_b || left_a > right_b || bottom_a < top_b || top_a > bottom_b)
    }

    /// Reçoit un coup depuis la position (from_x, from_y).
    pub fn take_hit(&mut self, from_x: f32, from_y: f32, damage: i32) {
        if self.is_invincible || !self.is_alive { return; }

        self.hp -= damage;

        if self.hp <= 0 {
            self.hp       = 0;
            self.is_alive = false;
            self.death_timer = 0.3; // durée de l'animation de mort
            return;
        }

        // Knockback
        self.is_invincible       = true;
        self.invincibility_timer = ENEMY_INVINCIBILITY;
        self.knockback_timer     = ENEMY_KNOCKBACK_DURATION;

        let dx  = self.x - from_x;
        let dy  = self.y - from_y;
        let len = (dx * dx + dy * dy).sqrt();

        if len > 0.001 {
            self.knockback_x = dx / len * ENEMY_KNOCKBACK_SPEED;
            self.knockback_y = dy / len * ENEMY_KNOCKBACK_SPEED;
        }
    }


}

// --- Fonctions utilitaires mathématiques ---

/// Distance euclidienne entre deux points.
fn distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
}

/// Normalise un vecteur (dx, dy) — retourne (0,0) si le vecteur est nul.
fn normalize(dx: f32, dy: f32) -> (f32, f32) {
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 { (0.0, 0.0) } else { (dx / len, dy / len) }
}