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

// Constantes pour les icônes objets dans objects.png
const OBJ_SPRITE_SIZE: u32  = 16;
const OBJ_DRAW_SIZE: u32    = 16;   // petite taille pour le HUD
const COL_RUBY: i32         = 1;    // colonne rubis dans objects.png
const COL_KEY:  i32         = 2;    // colonne clé dans objects.png
const COL_KEY_SILVER: i32   = 10;
const COL_KEY_GOLD:   i32   = 11;
const COL_KEY_BOSS:   i32   = 12;

/// Données précalculées pour le rendu du HUD.
/// À créer une seule fois dans main() et réutiliser chaque frame.
pub struct Hud<'a> {
    hearts_texture: Texture<'a>,
    objects_texture: Texture<'a>,
    label_texture: Texture<'a>,
    label_w: u32,
    label_h: u32,
    // total_hearts: i32,
    digits: Vec<(Texture<'a>, u32, u32)>,
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
        objects_path: &str,
        // _max_hp: i32,
    ) -> Result<Self, String> {
        use sdl2::image::LoadTexture;

        // Chargement du spritesheet de cœurs
        let hearts_texture = texture_creator
            .load_texture(hearts_path)
            .map_err(|e| format!("Impossible de charger '{hearts_path}': {e}"))?;

        let objects_texture = texture_creator.load_texture(objects_path)
            .map_err(|e| format!("Impossible de charger '{objects_path}': {e}"))?;

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

        // Pré-rendu des chiffres 0–99
        let mut digits = Vec::with_capacity(100);
        for n in 0..100u32 {
            let surface = font
                .render(&format!("x {n}"))
                .blended(COLOR_LABEL)
                .map_err(|e| format!("Erreur TTF digit {n}: {e}"))?;
            let w = surface.width();
            let h = surface.height();
            let tex = texture_creator
                .create_texture_from_surface(surface)
                .map_err(|e| format!("Erreur texture digit: {e}"))?;
            digits.push((tex, w, h));
        }

        // let total_hearts = (max_hp + 1) / 2;

        Ok(Hud {
            hearts_texture,
            objects_texture,
            label_texture,
            label_w,
            label_h,
            // total_hearts,
            digits,
        })
    }


    /// Dessine une icône + "x N" à la position donnée
    fn render_counter(
        &self,
        canvas: &mut Canvas<Window>,
        sprite_col: i32,
        count: i32,
        x: i32,
        y: i32,
    ) -> Result<(), String> {
        // Icône depuis objects.png
        let src = Rect::new(sprite_col * OBJ_SPRITE_SIZE as i32, 0, OBJ_SPRITE_SIZE, OBJ_SPRITE_SIZE);
        canvas.copy(&self.objects_texture,
                    Some(src),
                    Some(Rect::new(x, y, OBJ_DRAW_SIZE, OBJ_DRAW_SIZE)))?;

        // Compteur TTF pré-rendu
        let idx = count.clamp(0, 99) as usize;
        let (ref tex, w, h) = self.digits[idx];
        canvas.copy(
            tex,
            None,
            Some(Rect::new(
                x + OBJ_DRAW_SIZE as i32 + 3,
                y + (OBJ_DRAW_SIZE as i32 - h as i32) / 2,
                w,
                h
            )),
        )?;

        Ok(())
    }


    /// Dessine le HUD complet : panneau + label + cœurs.
    pub fn render(
        &self,
        canvas: &mut Canvas<Window>,
        hp: i32,
        max_hp: i32,
        rubies: i32,
        keys_basic: i32,
        keys_silver: i32,
        keys_gold: i32,
        keys_boss: i32,
        heart_pieces: u8,
        ) -> Result<(), String> {
        // --- Calcul des dimensions du panneau ---
        let total_hearts = (max_hp + 1) / 2;
        let hearts_total_w = total_hearts * (HEART_DRAW_W as i32 + HEART_GAP) - HEART_GAP;
        let content_w = self.label_w.max(hearts_total_w as u32);

        // Hauteur étendue pour rubis + clés
        let row_h     = OBJ_DRAW_SIZE + 4;

        let extra_key_rows = [
            (keys_silver, COL_KEY_SILVER),
            (keys_gold,   COL_KEY_GOLD),
            (keys_boss,   COL_KEY_BOSS),
        ]
            .iter()
            .filter(|(count, _)| *count > 0)
            .count() as u32;

        let heart_pieces_row = if heart_pieces > 0 { row_h } else { 0 };

        let panel_h   = self.label_h + HEART_DRAW_H
            + heart_pieces_row
            + row_h         // ligne rubis
            + row_h         // ligne clés
            + row_h * extra_key_rows
            + (PANEL_PAD_Y * (4 + extra_key_rows as i32 + if heart_pieces > 0 { 1 } else { 0 })) as u32;

        let panel_w   = content_w + (PANEL_PAD_X * 2) as u32;

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
        let hearts_x = PANEL_X + PANEL_PAD_X + (content_w as i32 - hearts_total_w) / 2;
        let hearts_y = label_y + self.label_h as i32 + PANEL_PAD_Y;

        for i in 0..total_hearts {
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

        // --- Pièces de cœur (si > 0) ---
        let pieces_y = hearts_y + HEART_DRAW_H as i32 + PANEL_PAD_Y;
        if heart_pieces > 0 {
            // Sprite HeartPiece = colonne 5 dans objects.png
            let src = Rect::new(5 * OBJ_SPRITE_SIZE as i32, 0, OBJ_SPRITE_SIZE, OBJ_SPRITE_SIZE);
            canvas.copy(
                &self.objects_texture,
                Some(src),
                Some(Rect::new(PANEL_X + PANEL_PAD_X, pieces_y, OBJ_DRAW_SIZE, OBJ_DRAW_SIZE)),
            )?;
            // Compteur "x N/4"
            let idx = heart_pieces.clamp(0, 3) as usize;
            // On réutilise digits mais on veut "1/4", "2/4", etc.
            // Option simple : afficher le chiffre seul avec digits[idx]
            let (ref tex, w, h) = self.digits[idx];
            canvas.copy(tex, None, Some(Rect::new(
                PANEL_X + PANEL_PAD_X + OBJ_DRAW_SIZE as i32 + 3,
                pieces_y + (OBJ_DRAW_SIZE as i32 - h as i32) / 2,
                w, h,
            )))?;
        }

        // --- Ligne rubis ---
        // let rubies_y = hearts_y + HEART_DRAW_H as i32 + PANEL_PAD_Y;
        let rubies_y = pieces_y + if heart_pieces > 0 { row_h as i32 } else { 0 };
        self.render_counter(canvas, COL_RUBY, rubies, PANEL_X + PANEL_PAD_X, rubies_y)?;

        // --- Ligne clé basic ---
        let keys_y = rubies_y + row_h as i32;
        self.render_counter(canvas, COL_KEY, keys_basic, PANEL_X + PANEL_PAD_X, keys_y)?;

        // --- Clés spéciales (affichées seulement si > 0) ---
        let mut next_y = keys_y + row_h as i32;
        for (count, col) in [
            (keys_silver, COL_KEY_SILVER),
            (keys_gold,   COL_KEY_GOLD),
            (keys_boss,   COL_KEY_BOSS),
        ] {
            if count > 0 {
                self.render_counter(canvas, col, count, PANEL_X + PANEL_PAD_X, next_y)?;
                next_y += row_h as i32;
            }
        }

        Ok(())
    }
}
