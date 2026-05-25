// src/player.rs
// Gère l'état, le mouvement et l'animation du joueur.

use sdl2::keyboard::{KeyboardState, Scancode};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::camera::Camera;
use crate::tilemap::{Tilemap, TILE_DRAW_SIZE};
use crate::tile_properties::TileTable;
use crate::npc::collides_with_npc_rect;
use crate::map_loader::KeyKind;

/// Vitesse de déplacement en pixels-monde par seconde
pub const PLAYER_SPEED: f32 = 180.0;
const INVINCIBILITY_DURATION: f32 = 0.8;
const KNOCKBACK_DURATION: f32 = 0.15;
const KNOCKBACK_SPEED: f32 = 260.0;
const PLAYER_MAX_HP: i32         = 6;
const ATTACK_DAMAGE: i32         = 2;
const ATTACK_DURATION: f32       = 0.25;  // durée de l'attaque en secondes
const ATTACK_RANGE: f32          = TILE_DRAW_SIZE as f32 * 0.9; // portée de l'épée

/// Taille native du sprite dans le spritesheet (pixels)
const SPRITE_W: u32 = 32;
const SPRITE_H: u32 = 32;

/// Nombre de frames par animation de direction
const ANIM_FRAMES: u32 = 8;

/// Durée d'une frame d'animation en secondes
const FRAME_DURATION: f32 = 0.12;

/// Demi-largeur et demi-hauteur de la hitbox en pixels-monde.
/// Plus petite que le sprite pour un meilleur feeling (comme Zelda LTTP).
/// Le sprite fait TILE_DRAW_SIZE px, la hitbox fait ~60% de cette taille.
const HITBOX_HALF_W: f32 = TILE_DRAW_SIZE as f32 * 0.30;
const HITBOX_HALF_H: f32 = TILE_DRAW_SIZE as f32 * 0.30;
pub const HITBOX_HALF: f32 = TILE_DRAW_SIZE as f32 * 0.30;

// src/player.rs — constantes d'animation d'attaque
const SLASH_SPRITE_W: u32   = 18;  // largeur d'une frame dans le spritesheet
const SLASH_SPRITE_H: u32   = 16;  // hauteur d'une frame
const SLASH_FRAMES: u32     = 4;   // nombre de frames d'animation
const SLASH_FRAME_DT: f32   = ATTACK_DURATION / SLASH_FRAMES as f32; // durée par frame
const SLASH_DRAW_SIZE: u32  = TILE_DRAW_SIZE;  // taille affichée

const SPIN_FRAME_DURATION: f32    = 0.12;   // durée de la rotation complète
const SPIN_SEQUENCE: [Direction; 8] = [  // séquence des directions
    Direction::Up,
    Direction::Left,
    Direction::Down,
    Direction::Right,
    Direction::Up,
    Direction::Left,
    Direction::Down,
    Direction::Right,
];

const VANISH_DURATION: f32  = 0.4;   // durée de l'animation vanish
const VANISH_FRAMES: u32    = 2;     // frames dans le spritesheet vanish
const VANISH_SPRITE_W: u32  = 16;
const VANISH_SPRITE_H: u32  = 16;
const VANISH_DRAW_SIZE: u32 = TILE_DRAW_SIZE;
const VANISH_FRAME_DT: f32  = VANISH_DURATION / VANISH_FRAMES as f32;

/// Les 4 directions possibles du joueur.
/// L'ordre correspond aux lignes dans le spritesheet (0=Bas, 1=Gauche, etc.)
#[derive(Clone, Copy, PartialEq)]
pub enum Direction {
    Down  = 0,
    Left  = 1,
    Right = 2,
    Up    = 3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeathState {
    Alive,
    Spinning,   // rotation 360°
    Vanishing,  // animation vanish
    Done,       // game over déclenché
}

/// Événements audio déclenchés pendant update()
#[derive(Default)]
pub struct PlayerAudioEvents {
    pub sword_swing:  bool,
    pub got_hit:      bool,
    pub death_start:  bool,
}

pub struct Player {
    /// Position du joueur dans le monde (centre du sprite, en pixels)
    pub x: f32,
    pub y: f32,

    /// Direction actuelle (détermine la ligne du spritesheet)
    pub direction: Direction,

    /// Est-ce que le joueur est en mouvement ?
    pub is_moving: bool,

    /// Index de la frame d'animation courante (0 à ANIM_FRAMES-1)
    anim_frame: u32,

    /// Accumulateur de temps pour l'animation
    anim_timer: f32,

    pub hp: i32,
    pub max_hp: i32,
    // pub is_alive: bool,
    pub is_invincible: bool,
    invincibility_timer: f32,
    knockback_x: f32,
    knockback_y: f32,
    knockback_timer: f32,
    pub death_state: DeathState,

    spin_timer: f32,     // temps écoulé dans la rotation
    vanish_timer: f32,   // temps restant dans le vanish
    vanish_frame: u32,   // frame courante du vanish


    // --- Attaque ---
    pub is_attacking: bool,
    attack_timer: f32,
    pub attack_anim_frame: u32,
    
    pub rubies: i32,
    pub keys_basic:  u32,
    pub keys_silver: u32,
    pub keys_gold:   u32,
    pub keys_boss:   u32,
    pub heart_pieces: u8,
}

impl Player {
    pub fn new(start_x: f32, start_y: f32) -> Self {
        Player {
            x: start_x,
            y: start_y,
            direction: Direction::Down,
            is_moving: false,
            anim_frame: 0,
            anim_timer: 0.0,
            hp: PLAYER_MAX_HP,
            max_hp: PLAYER_MAX_HP,
            // is_alive: true,
            is_invincible: false,
            invincibility_timer: 0.0,
            knockback_x: 0.0,
            knockback_y: 0.0,
            knockback_timer: 0.0,
            is_attacking: false,
            attack_timer: 0.0,
            attack_anim_frame: 0,
            death_state:  DeathState::Alive,
            spin_timer:   0.0,
            vanish_timer: 0.0,
            vanish_frame: 0,
            rubies: 0,
            keys_basic:  0,
            keys_silver: 0,
            keys_gold:   0,
            keys_boss:   0,
            heart_pieces: 0,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.death_state == DeathState::Alive
    }

    pub fn is_done(&self) -> bool {
        self.death_state == DeathState::Done
    }

    /// Mise à jour : lecture du clavier, déplacement, animation.
    /// Appelée une fois par frame avec le delta time.
    pub fn update(&mut self, dt: f32, kb: &KeyboardState, tilemap: &Tilemap, table: &TileTable, npcs: &[crate::npc::Npc]) -> PlayerAudioEvents {

        let mut events = PlayerAudioEvents::default();

        match self.death_state {
            DeathState::Alive => {
                if self.is_invincible {
                    self.invincibility_timer -= dt;
                    if self.invincibility_timer <= 0.0 {
                        self.invincibility_timer = 0.0;
                        self.is_invincible = false;
                    }
                }

                if self.attack_timer > 0.0 {
                    self.attack_timer -= dt;
                    // Calcul de la frame d'animation courante
                    let elapsed = ATTACK_DURATION - self.attack_timer;
                    self.attack_anim_frame =
                        ((elapsed / SLASH_FRAME_DT) as u32).min(SLASH_FRAMES - 1);

                    if self.attack_timer <= 0.0 {
                        self.attack_timer = 0.0;
                        self.is_attacking = false;
                        self.attack_anim_frame = 0;
                    }
                }

                // --- Knockback : prioritaire sur le déplacement normal ---
                if self.knockback_timer > 0.0 {
                    self.knockback_timer -= dt;

                    let next_x = self.x + self.knockback_x * dt;
                    if !self.collides_horizontal(next_x, self.y, tilemap, table) {
                        self.x = next_x;
                    }

                    let next_y = self.y + self.knockback_y * dt;
                    if !self.collides_vertical(self.x, next_y, tilemap, table) {
                        self.y = next_y;
                    }
                    return events; // Pas de contrôle joueur pendant le knockback
                }

                // --- Déclenchement attaque (touche Espace ou J) ---
                if !self.is_attacking
                    && (kb.is_scancode_pressed(Scancode::Space)
                    || kb.is_scancode_pressed(Scancode::J))
                {
                    self.is_attacking = true;
                    self.attack_timer = ATTACK_DURATION;
                    events.sword_swing = true;
                }

                // --- Déplacement (bloqué pendant l'attaque dans le style Zelda LTTP) ---
                if !self.is_attacking {
                    let mut dx = 0.0f32;
                    let mut dy = 0.0f32;

                    // Lecture des touches directionnelles (ZQSD + flèches)
                    if kb.is_scancode_pressed(Scancode::Up) || kb.is_scancode_pressed(Scancode::W) { dy -= 1.0; }
                    if kb.is_scancode_pressed(Scancode::Down) || kb.is_scancode_pressed(Scancode::S) { dy += 1.0; }
                    if kb.is_scancode_pressed(Scancode::Left) || kb.is_scancode_pressed(Scancode::A) { dx -= 1.0; }
                    if kb.is_scancode_pressed(Scancode::Right) || kb.is_scancode_pressed(Scancode::D) { dx += 1.0; }

                    self.is_moving = dx != 0.0 || dy != 0.0;

                    // --- Mise à jour de la direction ---
                    // On privilégie l'axe vertical (comme dans Zelda LTTP)
                    if dy < 0.0 { self.direction = Direction::Up; } else if dy > 0.0 { self.direction = Direction::Down; } else if dx < 0.0 { self.direction = Direction::Left; } else if dx > 0.0 { self.direction = Direction::Right; }

                    // --- Normalisation diagonale ---
                    if dx != 0.0 && dy != 0.0 {
                        let factor = 1.0 / std::f32::consts::SQRT_2;
                        dx *= factor;
                        dy *= factor;
                    }

                    let next_x = self.x + dx * PLAYER_SPEED * dt;
                    if !self.collides_horizontal(next_x, self.y, tilemap, table)
                        && !collides_with_npc_rect(next_x, self.y, HITBOX_HALF_W, HITBOX_HALF_H, npcs)
                    {
                        self.x = next_x;
                    }

                    let next_y = self.y + dy * PLAYER_SPEED * dt;
                    if !self.collides_vertical(self.x, next_y, tilemap, table)
                        && !collides_with_npc_rect(self.x, next_y, HITBOX_HALF_W, HITBOX_HALF_H, npcs)
                    {
                        self.y = next_y;
                    }
                } else {
                    self.is_moving = false;
                }

                // --- Animation ---
                if self.is_moving {
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

            DeathState::Spinning => {
                self.spin_timer += dt;

                // Quelle direction afficher selon le temps écoulé ?
                let frame_idx = (self.spin_timer / SPIN_FRAME_DURATION) as usize;

                if frame_idx < SPIN_SEQUENCE.len() {
                    // On force la direction du sprite sans bouger le joueur
                    self.direction = SPIN_SEQUENCE[frame_idx];
                } else {
                    // Séquence terminée → vanish
                    self.death_state  = DeathState::Vanishing;
                    self.vanish_timer = VANISH_DURATION;
                    self.vanish_frame = 0;
                }
            }

            DeathState::Vanishing => {
                self.vanish_timer -= dt;

                // Calcul de la frame vanish
                let elapsed = VANISH_DURATION - self.vanish_timer;
                self.vanish_frame =
                    ((elapsed / VANISH_FRAME_DT) as u32).min(VANISH_FRAMES - 1);

                if self.vanish_timer <= 0.0 {
                    self.death_state = DeathState::Done;
                }
            }

            DeathState::Done => {}
        }

        events
    }

    /// Reçoit un coup : dégâts + knockback + invincibilité.
    pub fn take_hit(&mut self, from_x: f32, from_y: f32, damage: i32) {

        // println!("→ take_hit : hp={} damage={} invincible={}", self.hp, damage, self.is_invincible);

        if !self.is_alive() || self.is_invincible { return; }

        self.hp -= damage;

        // println!("→ hp restant : {}", self.hp);

        if self.hp <= 0 {
            self.hp       = 0;
            self.death_state = DeathState::Spinning;  // ← déclenche l'animation
            // println!("→ MORT : death_state = Spinning");
            self.spin_timer  = 0.0;
            return;
        }

        self.is_invincible       = true;
        self.invincibility_timer = INVINCIBILITY_DURATION;
        self.knockback_timer     = KNOCKBACK_DURATION;

        let dx  = self.x - from_x;
        let dy  = self.y - from_y;
        let len = (dx * dx + dy * dy).sqrt();

        if len > 0.001 {
            self.knockback_x = dx / len * KNOCKBACK_SPEED;
            self.knockback_y = dy / len * KNOCKBACK_SPEED;
        } else {
            self.knockback_x = 0.0;
            self.knockback_y = KNOCKBACK_SPEED;
        }

    }

    /// Retourne la hitbox de l'épée si le joueur attaque, sinon None.
    /// La hitbox est positionnée devant le joueur selon sa direction.
    pub fn attack_hitbox(&self) -> Option<(f32, f32, f32, f32)> {
        if !self.is_attacking { return None; }

        let hw = TILE_DRAW_SIZE as f32 * 0.5;
        let hh = TILE_DRAW_SIZE as f32 * 0.3;

        let (ax, ay) = match self.direction {
            Direction::Up    => (self.x,                    self.y - ATTACK_RANGE),
            Direction::Down  => (self.x,                    self.y + ATTACK_RANGE),
            Direction::Left  => (self.x - ATTACK_RANGE,     self.y),
            Direction::Right => (self.x + ATTACK_RANGE,     self.y),
        };

        Some((ax, ay, hw, hh))
    }


    /// Vérifie si la hitbox à la position (next_x, y) entre en collision
    /// avec une tuile solide sur l'axe horizontal.
    /// On teste les coins gauche ou droit selon la direction.
    fn collides_horizontal(&self, next_x: f32, y: f32, tilemap: &Tilemap, table: &TileTable) -> bool {
        // Coin avant-haut et avant-bas dans la direction X
        let check_x = if next_x > self.x {
            next_x + HITBOX_HALF_W  // On va à droite : tester le bord droit
        } else {
            next_x - HITBOX_HALF_W  // On va à gauche : tester le bord gauche
        };

        // Tester les deux coins verticaux (haut et bas de la hitbox)
        tilemap.is_solid_at(check_x, y - HITBOX_HALF_H + 1.0, table)
            || tilemap.is_solid_at(check_x, y + HITBOX_HALF_H - 1.0, table)
    }

    /// Vérifie si la hitbox à la position (x, next_y) entre en collision
    /// avec une tuile solide sur l'axe vertical.
    fn collides_vertical(&self, x: f32, next_y: f32, tilemap: &Tilemap, table: &TileTable) -> bool {
        // Coin avant-gauche et avant-droit dans la direction Y
        let check_y = if next_y > self.y {
            next_y + HITBOX_HALF_H  // On va vers le bas : tester le bord bas
        } else {
            next_y - HITBOX_HALF_H  // On va vers le haut : tester le bord haut
        };

        // Tester les deux coins horizontaux (gauche et droite de la hitbox)
        tilemap.is_solid_at(x - HITBOX_HALF_W + 1.0, check_y, table)
            || tilemap.is_solid_at(x + HITBOX_HALF_W - 1.0, check_y, table)
    }

    pub fn attack_damage(&self) -> i32 { ATTACK_DAMAGE }

    pub fn clamp_to_map(&mut self, map_w: f32, map_h: f32) {
        let h = HITBOX_HALF;
        self.x = self.x.clamp(h, map_w - h);
        self.y = self.y.clamp(h, map_h - h);
    }

    /// Rendu du joueur à l'écran.
    /// Le sprite est agrandi avec le même TILE_SCALE que les tuiles.
    pub fn render(
        &self,
        canvas: &mut Canvas<Window>,
        spritesheet: &Texture,
        slash_sheet: &Texture,
        vanish_sheet: &Texture,
        camera: &Camera,
    ) -> Result<(), String> {
        match self.death_state {
            DeathState::Alive => {

                // Clignotement pendant l'invincibilité
                if self.is_invincible && (self.invincibility_timer * 20.0) as i32 % 2 == 0 {
                    return Ok(());
                }

                self.render_player_sprite(canvas, spritesheet, camera, 0.0)?;

                // Dessin de l'épée pendant l'attaque

                if self.is_attacking {
                    self.render_slash(canvas, slash_sheet, camera)?;
                }
            }

            DeathState::Spinning => {
                // On utilise render_player_sprite sans rotation (angle = 0.0)
                // La direction est déjà mise à jour dans update()
                // On peut ajouter un léger clignotement pour l'effet dramatique
                let blink = (self.spin_timer * 15.0) as i32 % 2 == 0;
                if !blink {
                    self.render_player_sprite(canvas, spritesheet, camera, 0.0)?;
                }
            }

            DeathState::Vanishing => {
                // Sprite vanish centré sur la position du joueur
                let src = Rect::new(
                    (self.vanish_frame * VANISH_SPRITE_W) as i32,
                    0,
                    VANISH_SPRITE_W,
                    VANISH_SPRITE_H,
                );
                let half = (VANISH_DRAW_SIZE / 2) as f32;
                let (sx, sy) = camera.world_to_screen(self.x - half, self.y - half);
                canvas.copy(
                    vanish_sheet,
                    Some(src),
                    Some(Rect::new(sx, sy, VANISH_DRAW_SIZE, VANISH_DRAW_SIZE)),
                )?;
            }

            DeathState::Done => {}
        }

        Ok(())
    }

    /// Dessine le sprite du joueur avec une rotation optionnelle.
    fn render_player_sprite(
        &self,
        canvas: &mut Canvas<Window>,
        spritesheet: &Texture,
        camera: &Camera,
        angle: f64,
    ) -> Result<(), String> {
        let src = Rect::new(
            (self.anim_frame * SPRITE_W) as i32,
            (self.direction as u32 * SPRITE_H) as i32,
            SPRITE_W,
            SPRITE_H,
        );
        let half = (TILE_DRAW_SIZE / 2) as f32;
        let (sx, sy) = camera.world_to_screen(self.x - half, self.y - half);
        let dst = Rect::new(sx, sy, TILE_DRAW_SIZE, TILE_DRAW_SIZE);

        // copy_ex pour supporter la rotation
        canvas.copy_ex(spritesheet, Some(src), Some(dst), angle, None, false, false)?;

        Ok(())
    }


    /// Dessine la hitbox en rouge semi-transparent (debug uniquement).
    /// À supprimer avant la version finale.
    pub fn render_hitbox(
        &self,
        canvas: &mut Canvas<Window>,
        camera: &Camera,
    ) -> Result<(), String> {
        use sdl2::rect::Rect;

        let (sx, sy) = camera.world_to_screen(
            self.x - HITBOX_HALF_W,
            self.y - HITBOX_HALF_H,
        );

        canvas.set_draw_color(Color::RGBA(255, 0, 0, 180));
        canvas.draw_rect(Rect::new(
            sx, sy,
            (HITBOX_HALF_W * 2.0) as u32,
            (HITBOX_HALF_H * 2.0) as u32,
        ))?;

        Ok(())
    }

    /// Dessine le sprite d'épée positionné et orienté devant le joueur.
    fn render_slash(
        &self,
        canvas: &mut Canvas<Window>,
        slash_sheet: &Texture,
        camera: &Camera,
    ) -> Result<(), String> {
        // Toutes les frames sur une seule ligne
        let src = Rect::new(
            (self.attack_anim_frame * SLASH_SPRITE_W) as i32,
            0,
            SLASH_SPRITE_W,
            SLASH_SPRITE_H,
        );

        // Rotation selon la direction
        let angle = match self.direction {
            Direction::Down  =>   270.0,
            Direction::Up    => 90.0,
            Direction::Right =>  180.0,   // sens horaire
            Direction::Left  => 0.0,
        };


        // 1. DISTANCE devant le joueur (en pixels monde)
        //    Augmenter = épée plus loin, Diminuer = épée plus proche
        // Offset vertical (haut/bas) — augmentez pour éloigner l'épée du joueur
        let offset_v = TILE_DRAW_SIZE as f32 * 0.70;

        // Offset horizontal (gauche/droite) — diminuez pour rapprocher l'épée
        let offset_h = TILE_DRAW_SIZE as f32 * 0.60;

        let (slash_world_x, slash_world_y) = match self.direction {
            Direction::Down  => (self.x + 6.0, self.y + offset_v),
            Direction::Up    => (self.x - 10.0, self.y - offset_v),
            Direction::Left  => (self.x - offset_h,  self.y + 14.0),
            Direction::Right => (self.x + offset_h,  self.y - 8.0 ),
        };

        // 2. TAILLE affichée à l'écran
        let draw_w = ((SLASH_DRAW_SIZE as f64) * 0.7) as u32;   // ← largeur en pixels écran
        let draw_h = ((SLASH_DRAW_SIZE as f64) * 0.7) as u32;   // ← hauteur en pixels écran

        // 3. ANCRAGE du sprite (par défaut : centre du sprite)
        //    Changez half_w / half_h pour décaler le point d'ancrage
        let half_w = (draw_w / 2) as f32;  // ← ancrage X (0.0 = bord gauche)
        let half_h = (draw_h / 2) as f32;  // ← ancrage Y (0.0 = bord haut)

        let (sx, sy) = camera.world_to_screen(slash_world_x - half_w, slash_world_y - half_h);
        let dst = Rect::new(sx, sy, draw_w, draw_h);

        // copy_ex : src, dst, angle, pivot (None = centre), flip_h, flip_v
        canvas.copy_ex(
            slash_sheet,
            Some(src),
            Some(dst),
            angle,
            None,   // pivot au centre du sprite
            false,
            false,
        )?;

        Ok(())
    }

    pub fn has_key(&self, kind: KeyKind) -> bool {
        match kind {
            KeyKind::Basic  => self.keys_basic  > 0,
            KeyKind::Silver => self.keys_silver > 0,
            KeyKind::Gold   => self.keys_gold   > 0,
            KeyKind::Boss   => self.keys_boss   > 0,
        }
    }

    pub fn use_key(&mut self, kind: KeyKind) {
        match kind {
            KeyKind::Basic  => self.keys_basic  = self.keys_basic.saturating_sub(1),
            KeyKind::Silver => self.keys_silver = self.keys_silver.saturating_sub(1),
            KeyKind::Gold   => self.keys_gold   = self.keys_gold.saturating_sub(1),
            KeyKind::Boss   => self.keys_boss   = self.keys_boss.saturating_sub(1),
        }
    }

}

