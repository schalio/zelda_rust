// src/map_utils.rs

use std::collections::HashMap;
use crate::map_loader::{load_tmx, MapFile, MapObject, ObjectKind};
use crate::tile_properties::TileTable;

pub fn get_png_path_from_tsx(tsx_path: &str) -> Result<String, String> {
    let content = std::fs::read_to_string(tsx_path)
        .map_err(|e| format!("Impossible de lire '{tsx_path}': {e}"))?;

    let doc = roxmltree::Document::parse(&content)
        .map_err(|e| format!("Erreur XML: {e}"))?;

    let source = doc.root_element()
        .children()
        .find(|n| n.has_tag_name("image"))
        .and_then(|n| n.attribute("source"))
        .ok_or("Attribut 'source' manquant dans <image> du TSX")?;

    let tsx_dir = std::path::Path::new(tsx_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));

    Ok(tsx_dir.join(source).to_string_lossy().into_owned())
}

pub fn load_map(name: &str) -> Result<(MapFile, TileTable, String), String> {
    let path = format!("assets/maps/{name}.tmx");
    let map_file   = load_tmx(&path)?;
    let tile_table = TileTable::from_tsx(&map_file.tileset_path)?;
    let png_path   = get_png_path_from_tsx(&map_file.tileset_path)?;
    Ok((map_file, tile_table, png_path))
}

pub fn save_collected(
    objects: &[MapObject],
    map_name: &str,
    collected: &mut HashMap<String, Vec<(i32, i32)>>,
) {
    let coords: Vec<(i32, i32)> = objects.iter()
        .filter(|o| o.collected && matches!(
            o.kind,
            ObjectKind::HeartPiece
            | ObjectKind::Chest { .. }
            | ObjectKind::Transition { .. }
        ))
        .map(|o| (o.x as i32, o.y as i32))
        .collect();

    collected.insert(map_name.to_string(), coords);
}

pub fn apply_collected(
    objects: &mut Vec<MapObject>,
    map_name: &str,
    collected: &HashMap<String, Vec<(i32, i32)>>,
) {
    if let Some(coords) = collected.get(map_name) {
        for obj in objects.iter_mut() {
            if matches!(
                obj.kind,
                ObjectKind::HeartPiece
                | ObjectKind::Chest { .. }
                | ObjectKind::Transition { .. }
            ) {
                if coords.contains(&(obj.x as i32, obj.y as i32)) {
                    obj.collected = true;
                }
            }
        }
    }
}