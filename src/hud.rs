// src/hud.rs

use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::ttf::Font;
use sdl2::video::Window;

// --- Configuration visuelle (ajustez pour matcher votre image) ---
const HEART_W: u32       = 10;   // largeur d'un cœur dans le spritesheet
const HEART_H: u32       = 10;   // hauteur d'un cœur dans le spritesheet
const HEART_DRAW_W: u32  = 14;   // taille affichée à l'écran
const HEART_DRAW_H: u32  = 14;
const HEART_GAP: i32     = 2;    // espacement entre cœurs
const PANEL_PAD_X: i32   = 8;    // padding horizontal du panneau
const PANEL_PAD_Y: i32   = 6;    // padding vertical du panneau
const PANEL_X: i32       = 8;    // position X du panneau HUD
const PANEL_Y: i32       = 6;    // position Y du panneau HUD
pub const FONT_SIZE: u16     = 8;    // taille de la police (pixels)

// Couleurs du panneau (bleu-violet comme sur l'image)
const COLOR_PANEL_BG:     Color = Color { r: 30,  g: 20,  b: 180, a: 220 };
const COLOR_PANEL_BORDER: Color = Color { r: 100, g: 80,  b: 255, a: 255 };
const COLOR_LABEL:        Color = Color { r: 255, g: 255, b: 255, a: 255 };

/// Index des cœurs dans le spritesheet (colonnes)
const HEART_FULL:  i32 = 0;
const HEART_HALF:  i32 = 1;
const HEART_EMPTY: i32 = 2;

/// Données précalculées pour le rendu du HUD.
/// À créer une seule fois dans main() et réutiliser chaque frame.
pub struct Hud<'a> {
    hearts_texture: Texture<'a>,
    label_texture: Texture<'a>,
    label_w: u32,
    label_h: u32,
    total_hearts: i32,
}

impl<'a> Hud<'a> {
    /// Initialise le HUD : charge le spritesheet et pré-rend le label TTF.
    ///
    /// # Arguments
    /// * `texture_creator` : créateur de textures SDL2
    /// * `font`            : police chargée via sdl2::ttf
    /// * `hearts_path`     : chemin vers hearts.png
    /// * `max_hp`          : PV maximum du joueur
    pub fn new<T>(
        texture_creator: &'a TextureCreator<T>,
        font: &Font,
        hearts_path: &str,
        max_hp: i32,
    ) -> Result<Self, String> {
        use sdl2::image::LoadTexture;

        // Chargement du spritesheet de cœurs
        let hearts_texture = texture_creator
            .load_texture(hearts_path)
            .map_err(|e| format!("Impossible de charger '{hearts_path}': {e}"))?;

        // Rendu du label "— LIFE —" via TTF
        let surface = font
            .render("- LIFE -")
            .blended(COLOR_LABEL)
            .map_err(|e| format!("Erreur TTF render: {e}"))?;

        let label_w = surface.width();
        let label_h = surface.height();

        let label_texture = texture_creator
            .create_texture_from_surface(surface)
            .map_err(|e| format!("Erreur texture TTF: {e}"))?;

        let total_hearts = (max_hp + 1) / 2;

        Ok(Hud {
            hearts_texture,
            label_texture,
            label_w,
            label_h,
            total_hearts,
        })
    }

    /// Dessine le HUD complet : panneau + label + cœurs.
    pub fn render(&self, canvas: &mut Canvas<Window>, hp: i32) -> Result<(), String> {
        // --- Calcul des dimensions du panneau ---
        let hearts_total_w = self.total_hearts * (HEART_DRAW_W as i32 + HEART_GAP) - HEART_GAP;
        let content_w = self.label_w.max(hearts_total_w as u32);
        let panel_w   = content_w + (PANEL_PAD_X * 2) as u32;
        let panel_h   = self.label_h + HEART_DRAW_H + (PANEL_PAD_Y * 3) as u32;

        // --- Fond du panneau ---
        canvas.set_draw_color(COLOR_PANEL_BG);
        canvas.fill_rect(Rect::new(PANEL_X, PANEL_Y, panel_w, panel_h))?;

        // --- Bordure du panneau ---
        canvas.set_draw_color(COLOR_PANEL_BORDER);
        canvas.draw_rect(Rect::new(PANEL_X, PANEL_Y, panel_w, panel_h))?;
        // Double bordure (style rétro)
        canvas.draw_rect(Rect::new(PANEL_X + 1, PANEL_Y + 1, panel_w - 2, panel_h - 2))?;

        // --- Label "— LIFE —" centré ---
        let label_x = PANEL_X + PANEL_PAD_X + (content_w as i32 - self.label_w as i32) / 2;
        let label_y = PANEL_Y + PANEL_PAD_Y;
        canvas.copy(
            &self.label_texture,
            None,
            Some(Rect::new(label_x, label_y, self.label_w, self.label_h)),
        )?;

        // --- Cœurs centrés sous le label ---
        let hearts_x = PANEL_X + PANEL_PAD_X
            + (content_w as i32 - hearts_total_w) / 2;
        let hearts_y = label_y + self.label_h as i32 + PANEL_PAD_Y;

        for i in 0..self.total_hearts {
            let cx = hearts_x + i * (HEART_DRAW_W as i32 + HEART_GAP);
            let pv_left = hp - i * 2;

            let src_col = if pv_left >= 2 {
                HEART_FULL
            } else if pv_left == 1 {
                HEART_HALF
            } else {
                HEART_EMPTY
            };

            let src = Rect::new(src_col * HEART_W as i32, 0, HEART_W, HEART_H);
            let dst = Rect::new(cx, hearts_y, HEART_DRAW_W, HEART_DRAW_H);

            canvas.copy(&self.hearts_texture, Some(src), Some(dst))?;
        }

        Ok(())
    }
}