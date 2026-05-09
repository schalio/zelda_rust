// src/map_loader.rs

use crate::tilemap::{Tilemap, TileIndex, EMPTY_TILE};

pub struct MapFile {
    pub layers: Vec<TiledLayer>,
    pub tileset_path: String,
    pub firstgid: u32,
}

pub struct TiledLayer {
    pub name: String,
    pub tilemap: Tilemap,
}

pub fn load_tmx(path: &str) -> Result<MapFile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Impossible de lire '{path}': {e}"))?;

    let doc = roxmltree::Document::parse(&content)
        .map_err(|e| format!("Erreur XML dans '{path}': {e}"))?;

    let root = doc.root_element();

    let map_width: usize = root
        .attribute("width")
        .and_then(|v| v.parse().ok())
        .ok_or("Attribut 'width' manquant")?;

    let map_height: usize = root
        .attribute("height")
        .and_then(|v| v.parse().ok())
        .ok_or("Attribut 'height' manquant")?;

    let tileset_node = root
        .children()
        .find(|n| n.has_tag_name("tileset"))
        .ok_or("Aucun <tileset> trouvé")?;

    let firstgid: u32 = tileset_node
        .attribute("firstgid")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let tsx_relative = tileset_node
        .attribute("source")
        .ok_or("Attribut 'source' manquant dans <tileset>")?;

    let tileset_path = resolve_relative_path(path, tsx_relative);

    let mut layers: Vec<TiledLayer> = Vec::new();

    for layer_node in root.children().filter(|n| n.has_tag_name("layer")) {
        let layer_name = layer_node
            .attribute("name")
            .unwrap_or("Sans nom")
            .to_string();

        let data_node = match layer_node.children().find(|n| n.has_tag_name("data")) {
            Some(n) => n,
            None    => continue,
        };

        let encoding = data_node.attribute("encoding").unwrap_or("");
        if encoding != "csv" {
            return Err(format!(
                "Couche '{layer_name}' : encodage '{encoding}' non supporté. \
                 Choisissez CSV dans Tiled."
            ));
        }

        let csv_text = data_node.text().unwrap_or("").trim();
        let tiles = parse_csv_layer(csv_text, map_width, map_height, firstgid)?;

        layers.push(TiledLayer {
            name: layer_name,
            tilemap: Tilemap::new(tiles),
        });
    }

    if layers.is_empty() {
        return Err(format!("Aucune couche trouvée dans '{path}'"));
    }

    Ok(MapFile { layers, tileset_path, firstgid })
}

fn parse_csv_layer(
    csv: &str,
    expected_w: usize,
    expected_h: usize,
    firstgid: u32,
) -> Result<Vec<Vec<TileIndex>>, String> {
    let mut rows: Vec<Vec<TileIndex>> = Vec::with_capacity(expected_h);

    for (row_idx, line) in csv.lines().enumerate() {
        let line = line.trim().trim_end_matches(',');
        if line.is_empty() {
            continue;
        }

        let row: Vec<TileIndex> = line
            .split(',')
            .map(|s| {
                let raw: u32 = s.trim().parse().unwrap_or(0);
                if raw == 0 {
                    EMPTY_TILE
                } else {
                    raw.saturating_sub(firstgid)
                }
            })
            .collect();

        if row.len() != expected_w {
            return Err(format!(
                "Ligne {row_idx} : {} colonnes, attendu {expected_w}",
                row.len()
            ));
        }

        rows.push(row);
    }

    if rows.len() != expected_h {
        return Err(format!(
            "{} lignes trouvées, attendu {expected_h}",
            rows.len()
        ));
    }

    Ok(rows)
}

fn resolve_relative_path(parent_path: &str, relative: &str) -> String {
    let parent_dir = std::path::Path::new(parent_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));

    parent_dir
        .join(relative)
        .to_string_lossy()
        .into_owned()
}