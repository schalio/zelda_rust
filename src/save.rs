// src/save.rs

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

const SAVE_DIR: &str = "saves";

#[derive(Serialize, Deserialize, Clone)]
pub struct SaveData {
    pub slot:              u8,
    pub hp:                i32,
    pub max_hp:            i32,
    pub rubies:            i32,
    pub keys_basic:        u32,
    pub keys_silver:       u32,
    pub keys_gold:         u32,
    pub keys_boss:         u32,
    pub current_map:       String,
    pub collected_objects: HashMap<String, Vec<(i32, i32)>>,
}

fn slot_path(slot: u8) -> String {
    format!("{SAVE_DIR}/save_{slot}.json")
}

/// Crée le dossier saves/ si nécessaire et écrit le fichier.
pub fn save_game(data: &SaveData) -> Result<(), String> {
    std::fs::create_dir_all(SAVE_DIR)
        .map_err(|e| format!("Impossible de créer '{SAVE_DIR}': {e}"))?;

    let json = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Erreur sérialisation: {e}"))?;

    std::fs::write(slot_path(data.slot), json)
        .map_err(|e| format!("Impossible d'écrire la sauvegarde: {e}"))?;

    println!("💾 Sauvegarde slot {} OK", data.slot);
    Ok(())
}

/// Charge un slot. Retourne None si le fichier n'existe pas.
pub fn load_game(slot: u8) -> Result<Option<SaveData>, String> {
    let path = slot_path(slot);

    if !std::path::Path::new(&path).exists() {
        return Ok(None);
    }

    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Impossible de lire '{path}': {e}"))?;

    let data = serde_json::from_str(&json)
        .map_err(|e| format!("Erreur désérialisation: {e}"))?;

    Ok(Some(data))
}

/// Retourne les métadonnées des 3 slots (None si vide).
pub fn list_slots() -> [Option<SaveData>; 3] {
    [
        load_game(1).unwrap_or(None),
        load_game(2).unwrap_or(None),
        load_game(3).unwrap_or(None),
    ]
}

/// Supprime un slot.
pub fn delete_save(slot: u8) -> Result<(), String> {
    let path = slot_path(slot);
    if std::path::Path::new(&path).exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Impossible de supprimer: {e}"))?;
    }
    Ok(())
}