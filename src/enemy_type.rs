// src/enemy_type.rs

/// Les différents types d'ennemis disponibles.
/// Chaque type correspond à une ligne dans le spritesheet des ennemis.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EnemyKind {
    Slime,   // Lent, faible, patrouille courte
    Goblin,  // Rapide, agressif, grande zone de détection
    Knight,  // Lent mais solide, patrouille longue
}

/// Statistiques associées à chaque type d'ennemi.
pub struct EnemyStats {
    pub max_hp: i32,
    pub speed: f32,           // pixels-monde par seconde
    pub detection_range: f32, // distance à laquelle il détecte le joueur (pixels)
    pub patrol_range: f32,    // amplitude de la patrouille (pixels)
    /// Ligne dans le spritesheet ennemi (0=Slime, 1=Goblin, 2=Knight)
    pub sprite_row: u32,
    /// Couleur de debug (R, G, B) utilisée pour le carré temporaire
    pub debug_color: (u8, u8, u8),
}

impl EnemyKind {
    /// Retourne les stats correspondant à ce type d'ennemi.
    pub fn stats(&self) -> EnemyStats {
        match self {
            EnemyKind::Slime => EnemyStats {
                max_hp: 2,
                speed: 60.0,
                detection_range: 150.0,
                patrol_range: 80.0,
                sprite_row: 0,
                debug_color: (80, 200, 80),   // vert
            },
            EnemyKind::Goblin => EnemyStats {
                max_hp: 4,
                speed: 130.0,
                detection_range: 170.0,
                patrol_range: 120.0,
                sprite_row: 1,
                debug_color: (200, 140, 40),  // orange
            },
            EnemyKind::Knight => EnemyStats {
                max_hp: 8,
                speed: 50.0,
                detection_range: 100.0,
                patrol_range: 160.0,
                sprite_row: 2,
                debug_color: (140, 140, 200), // bleu-gris
            },
        }
    }
}
