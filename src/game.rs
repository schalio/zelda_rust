// src/game.rs

use std::collections::HashMap;
use crate::transition::IrisTransition;

// ============================================================
// AppState — résultat retourné par main_menu()
// ============================================================

#[derive(Debug, PartialEq)]
pub enum AppState {
    Play,
    Quit,
}

// ============================================================
// DialogueState
// ============================================================

#[derive(Debug, Default)]
pub struct DialogueState {
    pub pages: Vec<String>,
    pub current_page: usize,
    pub close_cooldown: u32,
}

impl DialogueState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_active(&self) -> bool {
        !self.pages.is_empty()
    }

    pub fn close(&mut self) {
        self.pages.clear();
        self.current_page = 0;
        self.close_cooldown = 30;
    }

    pub fn tick_cooldown(&mut self) {
        if self.close_cooldown > 0 {
            self.close_cooldown -= 1;
        }
    }

    pub fn can_open(&self) -> bool {
        self.close_cooldown == 0
    }
}

// ============================================================
// Game
// ============================================================

pub struct Game {
    pub current_map_name: String,
    pub dialogue: DialogueState,
    pub flash_timer: f32,
    pub interact_pressed_last_frame: bool,
    pub iris: IrisTransition,
    pub collected_objects: HashMap<String, Vec<(i32, i32)>>,
    pub active_save_slot: u8,
}

impl Game {
    pub fn new() -> Self {
        Self {
            current_map_name: "zelda_test".to_string(),
            dialogue: DialogueState::new(),
            flash_timer: 0.0,
            interact_pressed_last_frame: false,
            iris: IrisTransition::new(),
            collected_objects: HashMap::new(),
            active_save_slot: 0,
        }
    }

    // Dans impl Game :

    pub fn to_save(&self, slot: u8, player: &crate::player::Player) -> crate::save::SaveData {
        crate::save::SaveData {
            slot,
            hp:                player.hp,
            max_hp:            player.max_hp,
            rubies:            player.rubies,
            keys_basic:        player.keys_basic,
            keys_silver:       player.keys_silver,
            keys_gold:         player.keys_gold,
            keys_boss:         player.keys_boss,
            current_map:       self.current_map_name.clone(),
            collected_objects: self.collected_objects.clone(),
        }
    }

    pub fn apply_save(&mut self, data: &crate::save::SaveData, player: &mut crate::player::Player) {
        player.hp          = data.hp;
        player.max_hp      = data.max_hp;
        player.rubies      = data.rubies;
        player.keys_basic  = data.keys_basic;
        player.keys_silver = data.keys_silver;
        player.keys_gold   = data.keys_gold;
        player.keys_boss   = data.keys_boss;
        self.current_map_name    = data.current_map.clone();
        self.collected_objects   = data.collected_objects.clone();
        self.active_save_slot    = data.slot;
    }


}