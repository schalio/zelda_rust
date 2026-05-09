// src/tile_properties.rs

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum TileKind {
    Walkable,
    Solid,
    Water,
    Door,
    Stairs,
}

impl TileKind {
    pub fn is_blocking(&self) -> bool {
        matches!(self, TileKind::Solid | TileKind::Water | TileKind::Door)
    }

    /// Construit un TileKind depuis la valeur de la propriété "kind" dans le TSX.
    fn from_str(s: &str) -> Self {
        match s {
            "solid"    => TileKind::Solid,
            "water"    => TileKind::Water,
            "door"     => TileKind::Door,
            "stairs"   => TileKind::Stairs,
            _          => TileKind::Walkable,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TileProperties {
    pub name: String,
    pub kind: TileKind,
}

pub struct TileTable {
    props: HashMap<u32, TileProperties>,
    pub tiles_per_row: u32,  // = columns dans le TSX
}

impl TileTable {
    /// Charge un fichier .tsx (Tiled Tileset XML) et construit la table.
    pub fn from_tsx(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Impossible de lire '{path}': {e}"))?;

        let doc = roxmltree::Document::parse(&content)
            .map_err(|e| format!("Erreur XML dans '{path}': {e}"))?;

        // --- Lecture de l'élément racine <tileset> ---
        let root = doc.root_element();

        // columns = nombre de tuiles par ligne dans le PNG
        let tiles_per_row: u32 = root
            .attribute("columns")
            .and_then(|v| v.parse().ok())
            .unwrap_or(16);

        let mut props: HashMap<u32, TileProperties> = HashMap::new();

        // --- Parcourir tous les <tile id="..."> ---
        for tile_node in root.children().filter(|n| n.has_tag_name("tile")) {

            // Récupérer l'id de la tuile
            let id: u32 = match tile_node.attribute("id").and_then(|v| v.parse().ok()) {
                Some(v) => v,
                None    => continue,
            };

            let mut tile_name = format!("Tuile {id}");
            let mut tile_kind = TileKind::Walkable;

            // --- Parcourir <properties> → <property name=... value=...> ---
            if let Some(props_node) = tile_node
                .children()
                .find(|n| n.has_tag_name("properties"))
            {
                for prop in props_node.children().filter(|n| n.has_tag_name("property")) {
                    let name  = prop.attribute("name").unwrap_or("");
                    let value = prop.attribute("value").unwrap_or("");

                    match name {
                        "kind" => { tile_kind = TileKind::from_str(value); }
                        "name" => { tile_name = value.to_string(); }
                        _      => {}
                    }
                }
            }

            props.insert(id, TileProperties {
                name: tile_name,
                kind: tile_kind,
            });
        }

        Ok(TileTable { props, tiles_per_row })
    }

    /// Retourne les propriétés d'une tuile.
    /// Tuile absente du TSX = Walkable par défaut.
    pub fn get(&self, tile_index: u32) -> TileProperties {
        self.props.get(&tile_index).cloned().unwrap_or(TileProperties {
            name: format!("Tuile {tile_index}"),
            kind: TileKind::Walkable,
        })
    }

    pub fn is_blocking(&self, tile_index: u32) -> bool {
        self.get(tile_index).kind.is_blocking()
    }
}