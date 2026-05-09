// src/camera.rs
// La caméra définit quelle partie du monde est visible à l'écran.

/// Représente la caméra du jeu.
/// Elle stocke la position (coin supérieur gauche) de la vue dans le monde.
pub struct Camera {
    pub x: f32,  // Position X dans le monde (en pixels)
    pub y: f32,  // Position Y dans le monde (en pixels)
    pub width: u32,   // Largeur de la fenêtre (en pixels)
    pub height: u32,  // Hauteur de la fenêtre (en pixels)
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        Camera {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    /// Centre la caméra sur une position du monde (ex: le joueur).
    /// La caméra est clampée pour ne pas dépasser les bords de la carte.
    ///
    /// # Arguments
    /// * `target_x`, `target_y` : position cible (centre de la vue)
    /// * `map_pixel_width`, `map_pixel_height` : dimensions totales de la carte en pixels
    pub fn center_on(
        &mut self,
        target_x: f32,
        target_y: f32,
        map_pixel_width: f32,
        map_pixel_height: f32,
    ) {
        // On veut que la cible soit au centre de l'écran
        self.x = target_x - (self.width as f32 / 2.0);
        self.y = target_y - (self.height as f32 / 2.0);

        // Clamping : empêche la caméra de sortir des bords de la carte
        // Bord gauche et haut
        if self.x < 0.0 { self.x = 0.0; }
        if self.y < 0.0 { self.y = 0.0; }

        // Bord droit et bas
        let max_x = map_pixel_width - self.width as f32;
        let max_y = map_pixel_height - self.height as f32;
        if self.x > max_x { self.x = max_x; }
        if self.y > max_y { self.y = max_y; }
    }

    /// Convertit une position du monde en position à l'écran.
    /// Utilisé pour savoir où dessiner un objet.
    pub fn world_to_screen(&self, world_x: f32, world_y: f32) -> (i32, i32) {
        (
            (world_x - self.x) as i32,
            (world_y - self.y) as i32,
        )
    }
}