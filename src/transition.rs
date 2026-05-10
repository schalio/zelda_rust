use sdl2::pixels::PixelFormatEnum;
// src/transition.rs  (nouveau fichier)
use sdl2::render::{Texture, TextureCreator};
use sdl2::video::WindowContext;


/// Durée de chaque demi-transition en secondes
pub const FADE_DURATION: f32 = 0.4;
pub const IRIS_DURATION: f32 = 1.0;
pub const IRIS_MAX_RADIUS: u32 = 650; // doit dépasser la diagonale de l'écran (800x600 → ~1000)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionPhase {
    None,          // Pas de transition en cours
    FadeOut,
    BlackPause,// L'écran se noircit (map actuelle encore visible)
    FadeIn,        // L'écran se rouvre (nouvelle map déjà chargée)
}

pub struct FadeTransition {
    pub phase:    TransitionPhase,
    pub timer:    f32,         // temps écoulé dans la phase courante (0.0 → FADE_DURATION)
    pub alpha:    u8,          // alpha courant du rectangle noir (0–255)
    /// Données de transition à appliquer quand l'écran est noir
    pub pending:  Option<(String, String)>,  // (target_map, target_entry)
}

impl FadeTransition {
    pub fn new() -> Self {
        Self {
            phase:   TransitionPhase::None,
            timer:   0.0,
            alpha:   0,
            pending: None,
        }
    }

    /// Déclenche le fade out. Stocker la cible pour l'appliquer au moment noir.
    pub fn start(&mut self, target_map: String, target_entry: String) {
        if self.phase != TransitionPhase::None { return; }
        self.phase   = TransitionPhase::FadeOut;
        self.timer   = 0.0;
        self.alpha   = 0;
        self.pending = Some((target_map, target_entry));
    }

    /// Met à jour l'animation. Retourne `Some((map, entry))` au moment où
    /// l'écran est entièrement noir (une seule fois).
    pub fn update(&mut self, dt: f32) -> Option<(String, String)> {
        match self.phase {
            TransitionPhase::None => None,

            TransitionPhase::FadeOut => {
                self.timer += dt;
                let t     = (self.timer / FADE_DURATION).min(1.0);
                self.alpha = (t * 255.0) as u8;

                if self.timer >= FADE_DURATION {
                    // Écran entièrement noir → signaler le rechargement
                    self.alpha = 255;
                    self.phase = TransitionPhase::FadeIn;
                    self.timer = 0.0;
                    self.pending.take()   // retourne la cible UNE seule fois
                } else {
                    None
                }
            }

            TransitionPhase::BlackPause => { None}

            TransitionPhase::FadeIn => {
                self.timer += dt;
                let t     = (self.timer / FADE_DURATION).min(1.0);
                self.alpha = ((1.0 - t) * 255.0) as u8;

                if self.timer >= FADE_DURATION {
                    self.alpha = 0;
                    self.phase = TransitionPhase::None;
                }
                None
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.phase != TransitionPhase::None
    }
}

pub struct IrisTransition {
    pub phase:       TransitionPhase,
    pub timer:       f32,
    pub pending:     Option<(String, String)>,
    pub cur_radius:  u32,
}

impl IrisTransition {
    pub fn new() -> Self {
        Self {
            phase:      TransitionPhase::None,
            timer:      0.0,
            pending:    None,
            cur_radius: IRIS_MAX_RADIUS,
        }
    }

    pub fn start(&mut self, target_map: String, target_entry: String) {
        if self.phase != TransitionPhase::None { return; }
        self.phase      = TransitionPhase::FadeOut; // on réutilise FadeOut = "fermeture"
        self.timer      = 0.0;
        self.cur_radius = IRIS_MAX_RADIUS;
        self.pending    = Some((target_map, target_entry));
    }

    /// Met à jour et retourne la cible quand le cercle est fermé.
    pub fn update(&mut self, dt: f32) -> Option<(String, String)> {
        match self.phase {
            TransitionPhase::None => None,

            TransitionPhase::FadeOut => {
                self.timer += dt;
                let t = (self.timer / IRIS_DURATION).min(1.0);
                // Ease in : le cercle se ferme lentement puis s'accélère
                let t_ease = t * t;
                self.cur_radius = ((1.0 - t_ease) * IRIS_MAX_RADIUS as f32) as u32;

                if self.timer >= IRIS_DURATION {
                    self.cur_radius = 0;
                    self.phase      = TransitionPhase::BlackPause;
                    self.timer      = 0.0;
                    self.pending.take()
                } else {
                    None
                }
            }

            TransitionPhase::BlackPause => {
                self.timer += dt;
                self.cur_radius = 0;
                if self.timer >= 0.2 {  // 200ms d'écran noir
                    self.phase = TransitionPhase::FadeIn;
                    self.timer = 0.0;
                }
                None
            }
            TransitionPhase::FadeIn => {
                self.timer += dt;
                let t = (self.timer / IRIS_DURATION).min(1.0);
                // Ease out : le cercle s'ouvre vite puis ralentit
                let t_ease = 1.0 - (1.0 - t) * (1.0 - t);
                self.cur_radius = (t_ease * IRIS_MAX_RADIUS as f32) as u32;

                if self.timer >= IRIS_DURATION {
                    self.cur_radius = IRIS_MAX_RADIUS;
                    self.phase      = TransitionPhase::None;
                }
                None
            }
        }
    }

    pub fn is_active(&self) -> bool {
        self.phase != TransitionPhase::None
    }
}



/// Dessine un disque plein sur une texture RGBA.
/// Les pixels dans le cercle sont transparents (alpha=0),
/// les pixels hors du cercle sont noirs opaques (alpha=255).
pub fn create_iris_texture<'a>(
    texture_creator: &'a TextureCreator<WindowContext>,
    width: u32,
    height: u32,
    radius: u32,
) -> Result<Texture<'a>, String> {
    let cx = (width  / 2) as i32;
    let cy = (height / 2) as i32;
    let r  = radius as i32;

    let mut tex = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGBA8888, width, height)
        .map_err(|e| e.to_string())?;

    tex.with_lock(None, |buf: &mut [u8], pitch: usize| {
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let dx = x - cx;
                let dy = y - cy;
                let inside = dx * dx + dy * dy <= r * r;

                let offset = y as usize * pitch + x as usize * 4;
                // Format RGBA8888 en mémoire SDL2 : A B G R
                if inside {
                    // Transparent (trou dans le masque)
                    buf[offset]     = 0;   // A
                    buf[offset + 1] = 0;   // B
                    buf[offset + 2] = 0;   // G
                    buf[offset + 3] = 0;   // R
                } else {
                    // Noir opaque (cache)
                    buf[offset]     = 255; // A
                    buf[offset + 1] = 0;   // B
                    buf[offset + 2] = 0;   // G
                    buf[offset + 3] = 0;   // R
                }
            }
        }
    })?;

    tex.set_blend_mode(sdl2::render::BlendMode::Blend);
    Ok(tex)
}