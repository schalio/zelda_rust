// src/map_loader.rs

use crate::npc::{Npc, NpcKind};
use crate::tilemap::{Tilemap, TileIndex, EMPTY_TILE, TILE_SCALE};

pub struct MapFile {
    pub layers: Vec<TiledLayer>,
    pub tileset_path: String,
    pub firstgid: u32,
    pub spawn_points: Vec<SpawnPoint>,
    pub objects: Vec<MapObject>,
    pub npcs: Vec<Npc>,
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

    // Parsing du layer spawns (objectgroup)
    let mut spawn_points: Vec<SpawnPoint> = Vec::new();

    for og in root.children().filter(|n| n.has_tag_name("objectgroup")) {
        if og.attribute("name").unwrap_or("") != "spawns" { continue; }

        for obj in og.children().filter(|n| n.has_tag_name("object")) {
            let x = obj.attribute("x")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.0);
            let y = obj.attribute("y")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.0);

            // Tiled ≥ 1.9 : "class" | Tiled < 1.9 : "type"
            let class = obj.attribute("class")
                .or_else(|| obj.attribute("type"))
                .unwrap_or("");

            let kind = match class {
                "player" => SpawnKind::Player,
                "slime"  => SpawnKind::Enemy(crate::enemy_type::EnemyKind::Slime),
                "goblin" => SpawnKind::Enemy(crate::enemy_type::EnemyKind::Goblin),
                "knight" => SpawnKind::Enemy(crate::enemy_type::EnemyKind::Knight),
                other    => {
                    println!("⚠ spawn inconnu : '{other}' ignoré");
                    continue;
                }
            };

            spawn_points.push(SpawnPoint {
                x: x * TILE_SCALE as f32,
                y: y * TILE_SCALE as f32,
                kind,
                name: obj.attribute("name").unwrap_or("").to_string(),
            });
        }
    }

    let mut objects: Vec<MapObject> = Vec::new();
    let mut npcs: Vec<Npc> = Vec::new();

    for og in root.children().filter(|n| n.has_tag_name("objectgroup")) {
        if og.attribute("name").unwrap_or("") != "objects" { continue; }

        for obj in og.children().filter(|n| n.has_tag_name("object")) {
            let x = obj.attribute("x")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.0) * TILE_SCALE as f32;
            let y = obj.attribute("y")
                .and_then(|v| v.parse::<f32>().ok())
                .unwrap_or(0.0) * TILE_SCALE as f32;

            let class = obj.attribute("class")
                .or_else(|| obj.attribute("type"))
                .unwrap_or("");

            let _npc_kind_prop = obj
                .descendants()
                .find(|n| n.has_tag_name("property")
                    && n.attribute("name") == Some("npc_kind"))
                .and_then(|n| n.attribute("value"))
                .unwrap_or("villager");

            let _solid_prop = obj
                .descendants()
                .find(|n| n.has_tag_name("property")
                    && n.attribute("name") == Some("solid"))
                .and_then(|n| n.attribute("value"))
                .unwrap_or("true");

            if class == "npc" {
                let npc_kind_prop = obj
                    .descendants()
                    .find(|n| n.has_tag_name("property")
                        && n.attribute("name") == Some("npc_kind"))
                    .and_then(|n| n.attribute("value"))
                    .unwrap_or("villager");

                let solid_prop = obj
                    .descendants()
                    .find(|n| n.has_tag_name("property")
                        && n.attribute("name") == Some("solid"))
                    .and_then(|n| n.attribute("value"))
                    .unwrap_or("true");

                let dialogue_prop = if npc_kind_prop == "animal" {
                    obj.descendants()
                        .find(|n| n.has_tag_name("property")
                            && n.attribute("name") == Some("dialogue_animal"))
                        .and_then(|n| n.attribute("value"))
                        .unwrap_or("Meuh.")
                } else {
                    obj.descendants()
                        .find(|n| n.has_tag_name("property")
                            && n.attribute("name") == Some("dialogue"))
                        .and_then(|n| n.attribute("value"))
                        .unwrap_or("Bonjour !")
                };

                let npc_kind = match npc_kind_prop {
                    "animal" => NpcKind::Animal,
                    "guy" => NpcKind::Guy,
                    "girl" => NpcKind::Girl,
                    _ => NpcKind::Villager,
                };

                let solid = match solid_prop {
                    "false" => false,
                    _ => true,
                };

                // --- Patrouille (optionnelle) ---
                let patrol_dx = get_float_property(&obj, "patrol_dx");
                let patrol_dy = get_float_property(&obj, "patrol_dy");

                let (patrol_origin, patrol_target) = match (patrol_dx, patrol_dy) {
                    (Some(dx), Some(dy)) => (
                        Some((x, y)),
                        Some((x + dx * TILE_SCALE as f32, y + dy * TILE_SCALE as f32)),
                    ),
                    _ => (None, None),
                };


                npcs.push(Npc {
                    x,
                    y,
                    kind: npc_kind,
                    solid,
                    dialogue: dialogue_prop.to_string(),
                    anim_timer: 0.0,
                    anim_frame: 0,
                    patrol_origin,
                    patrol_target,
                    patrol_going : true,
                    direction: 0,
                });

                continue;
            }

            // Lecture de la propriété "contains" pour les coffres
            let contains_prop = obj
                .descendants()
                .find(|n| n.has_tag_name("property")
                    && n.attribute("name") == Some("contains"))
                .and_then(|n| n.attribute("value"))
                .unwrap_or("ruby");

            let kind = match class {
                "heart" => ObjectKind::Heart,
                "ruby"  => ObjectKind::Ruby(RubyKind::Green),
                "key"   => ObjectKind::Key,
                "bush"  => ObjectKind::Bush,
                "chest" => {
                    let loot = match contains_prop {
                        "heart" => LootKind::Heart,
                        "key"   => LootKind::Key,
                        _       => LootKind::Ruby,
                    };
                    ObjectKind::Chest { contains: loot }
                }
                "transition" => {
                    // Lecture des propriétés custom
                    let mut target_map   = String::new();
                    let mut target_entry = String::new();

                    for prop in obj.descendants().filter(|n| n.has_tag_name("property")) {
                        match prop.attribute("name").unwrap_or("") {
                            "target_map"   => target_map   = prop.attribute("value").unwrap_or("").to_string(),
                            "target_entry" => target_entry = prop.attribute("value").unwrap_or("").to_string(),
                            _ => {}
                        }
                    }

                    if target_map.is_empty() {
                        println!("⚠ transition sans 'target_map' ignorée");
                        continue;
                    }

                    ObjectKind::Transition { target_map, target_entry }
                }
                "heart_piece" => ObjectKind::HeartPiece,
                other => {
                    println!("⚠ objet inconnu : '{other}' ignoré");
                    continue;
                }
            };

            objects.push(MapObject { x, y, kind, collected: false });
        }
    }

    Ok(MapFile { layers, tileset_path, firstgid, spawn_points, objects, npcs })
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

/// Lit une propriété float custom sur un nœud XML d'objet Tiled.
/// Cherche <properties><property name="nom" value="..."/></properties>
fn get_float_property(node: &roxmltree::Node, name: &str) -> Option<f32> {
    node.children()
        .find(|n| n.has_tag_name("properties"))?
        .children()
        .find(|n| {
            n.has_tag_name("property")
                && n.attribute("name") == Some(name)
        })?
        .attribute("value")?
        .parse::<f32>()
        .ok()
}

#[derive(Debug, Clone, Copy)]
pub enum SpawnKind {
    Player,
    Enemy(crate::enemy_type::EnemyKind),
}

#[derive(Debug, Clone)]
pub struct SpawnPoint {
    pub x: f32,
    pub y: f32,
    pub kind: SpawnKind,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjectKind {
    Heart,
    Ruby(RubyKind),
    Key,
    Chest { contains: LootKind },
    Transition {target_map: String, target_entry: String},
    HeartPiece,
    Bush,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LootKind {
    Heart,
    Ruby,
    Key,
}

#[derive(Debug, Clone)]
pub struct MapObject {
    pub x: f32,
    pub y: f32,
    pub kind: ObjectKind,
    pub collected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RubyKind {
    Green,
    Blue,
    Red,
}