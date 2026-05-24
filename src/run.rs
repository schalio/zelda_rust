// src/run.rs

use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::keyboard::Keycode;
use sdl2::mixer;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::{Duration, Instant};

use crate::audio::{self, AudioManager};
use crate::camera::Camera;
use crate::combat::{self, resolve_bush_cut, resolve_enemy_contact, resolve_object_contact, resolve_player_attack};
use crate::enemy::Enemy;
use crate::game::Game;
use crate::hud::{Hud, FONT_SIZE};
use crate::map_loader::{ObjectKind, SpawnKind, KeyKind};
use crate::map_utils::{apply_collected, load_map, save_collected};
use crate::npc::{find_npc_in_front, render_npcs, update_npcs};
use crate::objects::{render_objects, resolve_chest_collision};
use crate::player::{self, DeathState, Player};
use crate::rendering::render_dialogue_box;
use crate::screens::game_over_screen;
use crate::transition::create_iris_texture;
use crate::utils::{is_in_front_of_player, separate_enemies, split_dialogue};

const WINDOW_WIDTH:  u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const TARGET_FPS:    u64 = 60;
const FRAME_DURATION_MICROS: u64 = 1_000_000 / TARGET_FPS;

pub fn run(
    mut canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    mut event_pump: &mut sdl2::EventPump,
    ttf_context: &sdl2::ttf::Sdl2TtfContext,
    save_slot: u8,
    save_data: Option<crate::save::SaveData>,
) -> Result<(), String> {


    let texture_creator = canvas.texture_creator();

    let mut audio = AudioManager::new()?;
    audio.play_music("assets/music/dungeon.ogg")?;

    // --- Chargement des textures ---
    let spritesheet = texture_creator.load_texture("assets/sprites/player.png")?;
    let enemy_sheet = texture_creator.load_texture("assets/sprites/enemies.png")?;
    let slash_sheet = texture_creator.load_texture("assets/sprites/sword_slash.png")?;
    let vanish_sheet = texture_creator.load_texture("assets/sprites/vanish.png")?;
    let objects_sheet = texture_creator.load_texture("assets/sprites/objects.png")?;
    let npc_texture = texture_creator.load_texture("assets/sprites/npcs.png")?;

    // -------------------------------------------------------
    // État du jeu
    // -------------------------------------------------------

    let mut game = Game::new();
    game.active_save_slot = save_slot;

    if save_data.is_none() {
        // Nouvelle partie : on efface l'éventuelle ancienne save du slot
        if let Err(e) = crate::save::delete_save(save_slot) {
            eprintln!("⚠ Impossible de supprimer l'ancienne save: {e}");
        }
    }

    // --- Map de départ ---
    let start_map = save_data.as_ref()
        .map(|d| d.current_map.clone())
        .unwrap_or_else(|| "zelda_test".to_string());

    // --- Chargement Tiled ---
    let (mut map_file, mut tile_table, tileset_png) = load_map("zelda_test")?;
    let mut tileset  = texture_creator.load_texture(&tileset_png)?;
    let mut objects = map_file.objects.clone();
    let mut npcs = map_file.npcs.clone();
    game.current_map_name = start_map.clone();

    let player_spawn = map_file.spawn_points.iter()
        .find(|sp| matches!(sp.kind, SpawnKind::Player))
        .expect("❌ Aucun spawn 'player' dans la map !");

    let mut player = Player::new(player_spawn.x, player_spawn.y);

    // --- Appliquer sauvegarde si Continuer ---
    if let Some(ref data) = save_data {
        game.apply_save(data, &mut player);
        apply_collected(&mut objects, &start_map, &game.collected_objects);
    }

    let mut enemies: Vec<Enemy> = map_file.spawn_points.iter()
        .filter_map(|sp| {
            if let SpawnKind::Enemy(kind) = sp.kind {
                Some(Enemy::new(sp.x, sp.y, kind))
            } else {
                None
            }
        })
        .collect();

    // --- Caméra ---
    let mut camera = Camera::new(WINDOW_WIDTH, WINDOW_HEIGHT);

    let mut last_frame_time = Instant::now();

    // --- Chargement de la police ---
    let font = ttf_context
        .load_font("assets/fonts/zelda.ttf", FONT_SIZE)
        .map_err(|e| format!("Impossible de charger la police: {e}"))?;

    // --- Création du HUD (une seule fois) ---
    let hud = Hud::new(
        &texture_creator,
        &font,
        "assets/sprites/hearts.png",
        "assets/sprites/objects.png",
        // player.max_hp,
    )?;

    // --- Flash sauvegarde ---
    let mut save_flash_timer = 0.0f32;

    let save_flash_surf = font
        .render("Sauvegarde !")
        .blended(Color::RGBA(255, 255, 100, 220))
        .map_err(|e| e.to_string())?;
    let mut save_flash_tex = texture_creator
        .create_texture_from_surface(&save_flash_surf)
        .map_err(|e| e.to_string())?;
    let save_flash_w = save_flash_tex.query().width;
    let save_flash_h = save_flash_tex.query().height;

    // ============================================================
    // Boucle principale
    // ============================================================

    'game_loop: loop {
        let now = Instant::now();
        let dt = now.duration_since(last_frame_time).as_secs_f32();
        game.flash_timer = (game.flash_timer - dt).max(0.0);
        save_flash_timer  = (save_flash_timer  - dt).max(0.0);
        last_frame_time = now;
        let mut pause_requested = false;

        // --- Événements ---
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'game_loop,

                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    pause_requested = true;
                }

                Event::KeyDown { keycode: Some(Keycode::M), .. } => {
                    if mixer::Music::is_paused() {
                        audio.resume_music();
                    } else {
                        audio.pause_music();
                    }
                }

                Event::KeyDown { keycode: Some(Keycode::F5), .. } => {
                    let save = game.to_save(game.active_save_slot, &player);
                    match crate::save::save_game(&save) {
                        Ok(_)  => save_flash_timer = 1.5,  // ← timer activé uniquement sur F5
                        Err(e) => eprintln!("⚠ Sauvegarde échouée: {e}"),
                    }
                }
                _ => {}
            }
        }

        if pause_requested {
            use crate::pause::{pause_screen, PauseResult};
            match pause_screen(canvas, event_pump, ttf_context)? {
                PauseResult::Resume => {}
                PauseResult::SaveAndResume => {
                    let save = game.to_save(game.active_save_slot, &player);
                    match crate::save::save_game(&save) {
                        Ok(_)  => save_flash_timer = 1.5,
                        Err(e) => eprintln!("⚠ Sauvegarde échouée: {e}"),
                    }
                }
                PauseResult::MainMenu | PauseResult::Quit => break 'game_loop,
            }
        }

        // --- DÉCLENCHEMENT de la transition ---
        // let _map_to_load = iris.update(dt);

        if !game.iris.is_active() && !game.dialogue.is_active() {
            game.dialogue.tick_cooldown();

            if game.dialogue.can_open() {
                for i in 0..objects.len() {
                    let obj = &objects[i];
                    if !combat::player_touches_object(&player, obj) { continue; }

                    let (target_map_val, target_entry_val, locked_val) =
                        if let ObjectKind::Transition { target_map, target_entry, locked } = &obj.kind {
                            (target_map.clone(), target_entry.clone(), locked.clone())
                        } else {
                            continue;
                        };

                    match locked_val {
                        None => {
                            objects[i].collected = true;
                            game.iris.start(target_map_val, target_entry_val);
                            break;
                        }
                        Some(key_kind) => {
                            if objects[i].collected || player.has_key(key_kind) {
                                if !objects[i].collected {
                                    // Première ouverture : consommer la clé et marquer
                                    player.use_key(key_kind);
                                    objects[i].collected = true;
                                }
                                game.iris.start(target_map_val, target_entry_val);
                            } else {
                                let msg = match key_kind {
                                    KeyKind::Basic  => "Cette porte est verrouillée.\nIl te faut une clé.",
                                    KeyKind::Silver => "Cette porte requiert une clé d'argent.",
                                    KeyKind::Gold   => "Cette porte requiert une clé d'or.",
                                    KeyKind::Boss   => "Cette porte mène au boss.\nIl te faut la clé du donjon.",
                                };
                                game.dialogue.pages = split_dialogue(msg);
                                game.dialogue.current_page = 0;

                                // Repousser le joueur
                                use crate::tilemap::TILE_DRAW_SIZE;
                                let push = TILE_DRAW_SIZE as f32 * 0.1;
                                match player.direction {
                                    player::Direction::Up    => player.y += push,
                                    player::Direction::Down  => player.y -= push,
                                    player::Direction::Left  => player.x += push,
                                    player::Direction::Right => player.x -= push,
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }

        // --- RECHARGEMENT au moment où l'écran est noir ---
        if let Some((target_map, target_entry)) = game.iris.update(dt) {
            save_collected(&objects, &game.current_map_name, &mut game.collected_objects);

            // 💾 Sauvegarde automatique à chaque transition
            let save = game.to_save(game.active_save_slot, &player);
            if let Err(e) = crate::save::save_game(&save) {
                eprintln!("⚠ Sauvegarde automatique échouée: {e}");
            }

            let (new_map, new_table, new_png) = load_map(&target_map)?;

            // Fermer le dialogue proprement lors du changement de map
            game.dialogue.pages.clear();
            game.dialogue.current_page = 0;

            let entry = new_map.spawn_points.iter()
                .find(|sp| sp.name == target_entry)
                .or_else(|| new_map.spawn_points.iter()
                    .find(|sp| matches!(sp.kind, SpawnKind::Player)))
                .expect(&format!("❌ Spawn '{target_entry}' introuvable"));

            player.x = entry.x;
            player.y = entry.y;

            tileset  = texture_creator.load_texture(&new_png)?;
            objects  = new_map.objects.clone();
            npcs = new_map.npcs.clone();
            apply_collected(&mut objects, &target_map, &game.collected_objects);
            game.current_map_name = target_map;

            enemies = new_map.spawn_points.iter()
                .filter_map(|sp| {
                    if let SpawnKind::Enemy(kind) = sp.kind {
                        Some(Enemy::new(sp.x, sp.y, kind))
                    } else { None }
                })
                .collect();

            tile_table = new_table;
            map_file   = new_map;
        }

        let ground_layer = &map_file.layers[0].tilemap;

        // --- Mise à jour ---
        let kb = event_pump.keyboard_state();

        let interact_pressed =
            kb.is_scancode_pressed(sdl2::keyboard::Scancode::E) || kb.is_scancode_pressed(sdl2::keyboard::Scancode::Return);

        let interact_just_pressed = interact_pressed && !game.interact_pressed_last_frame;
        game.interact_pressed_last_frame = interact_pressed;

        update_npcs(&mut npcs, dt, player.x, player.y, ground_layer, &tile_table, game.dialogue.is_active());

        if !game.iris.is_active() && !game.dialogue.is_active() {
            let player_events = player.update(dt, &kb, &ground_layer, &tile_table, &npcs);
            player.clamp_to_map(ground_layer.pixel_width(), ground_layer.pixel_height());
            if player_events.sword_swing {
                audio.play_sword();
            }
        }

        if interact_just_pressed {
            if game.dialogue.is_active() {
                // Avancer à la page suivante ou fermer
                game.dialogue.current_page += 1;
                if game.dialogue.current_page >= game.dialogue.pages.len() {
                    game.dialogue.close();

                    use crate::tilemap::TILE_DRAW_SIZE;
                    let push = TILE_DRAW_SIZE as f32 * 0.1;
                    match player.direction {
                        player::Direction::Up    => player.y += push,
                        player::Direction::Down  => player.y -= push,
                        player::Direction::Left  => player.x += push,
                        player::Direction::Right => player.x -= push,
                    }
                }
            } else if let Some(npc_index) = find_npc_in_front(player.x, player.y, player.direction, &npcs) {
                // if npcs[npc_index].waypoints.is_none() {
                game.dialogue.pages = split_dialogue(&npcs[npc_index].dialogue);
                game.dialogue.current_page = 0;
                // }
            } else {
                // Panneau
                for obj in &objects {
                    if let ObjectKind::Sign { text } = &obj.kind {
                        if is_in_front_of_player(player.x, player.y, player.direction, obj.x, obj.y) {
                            game.dialogue.pages = split_dialogue(text);
                            game.dialogue.current_page = 0;
                            break;
                        }
                    }
                }
            }
        }


        camera.center_on(
            player.x,
            player.y,
            ground_layer.pixel_width(),
            ground_layer.pixel_height(),
        );

        if player.is_alive() {
            for enemy in enemies.iter_mut() {
                enemy.update(dt, player.x, player.y, &ground_layer, &tile_table);
            }

            separate_enemies(&mut enemies, &ground_layer, &tile_table);
            resolve_enemy_contact(&mut player, &mut enemies, &audio);
            resolve_player_attack(&player, &mut enemies, &mut objects, &audio);
            resolve_bush_cut(&player, &mut objects);
            if resolve_object_contact(&mut player, &mut objects, &audio) {
                game.flash_timer = 0.3;
            }

            resolve_chest_collision(&mut player, &objects);

            // Nettoyer les ennemis morts dont l'animation est terminée
            enemies.retain(|e| e.is_alive || e.death_timer > 0.0);
        }
        // Vérifier game over
        if player.death_state == DeathState::Done {
            // Attente son de mort (existant)
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                if !mixer::Channel(audio::CHANNEL_PLAYER_DEATH).is_playing()
                    || Instant::now() > deadline { break; }
                std::thread::sleep(Duration::from_millis(30));
            }

            // Écran game over
            game_over_screen(&mut canvas, &texture_creator, &ttf_context, &mut event_pump)?;
            break 'game_loop;
        }

        // --- Rendu ---
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        // ground_layer.render(&mut canvas, &tileset, &camera)?;
        // 1. Tilemap
        for layer in &map_file.layers {
            layer.tilemap.render(
                &mut canvas,
                &tileset,
                &camera,
                tile_table.tiles_per_row,
            )?;
        }

        // 1.1 Objets au sol
        render_objects(&objects, &mut canvas, &objects_sheet, &camera)?;
        render_npcs(&mut canvas, &npc_texture, &camera, &npcs)?;
        // npc::render_npc_hitboxes(&npcs, &mut canvas, &camera)?;  // ← debug

        // 2. Ennemis
        for enemy in enemies.iter() {
            enemy.render(&mut canvas, &enemy_sheet, &vanish_sheet, &camera)?;
            // enemy.render_debug(&mut canvas, &camera)?; // ← pour debug
        }

        // 3. Joueur
        player.render(&mut canvas, &spritesheet, &slash_sheet, &vanish_sheet, &camera)?;
        // player.render_hitbox(&mut canvas, &camera)?; // ← debug

        // Flash de collecte
        if game.flash_timer > 0.0 {
            let alpha = ((game.flash_timer / 0.3) * 180.0) as u8;
            canvas.set_draw_color(Color::RGBA(255, 255, 255, alpha));
            canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
            canvas.fill_rect(Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT))?;
            canvas.set_blend_mode(sdl2::render::BlendMode::None);
        }

        // 4. HUD — toujours en dernier, par-dessus tout
        hud.render(
            &mut canvas,
            player.hp,
            player.max_hp,
            player.rubies,
            player.keys_basic  as i32,
            player.keys_silver as i32,
            player.keys_gold   as i32,
            player.keys_boss   as i32,
        )?;

        if game.dialogue.is_active() {
            let is_last = game.dialogue.current_page >= game.dialogue.pages.len() - 1;
            render_dialogue_box(
                &mut canvas, &texture_creator, &font,
                &game.dialogue.pages[game.dialogue.current_page],
                is_last,
            )?;
        }

        if game.iris.is_active() {
            let mask = create_iris_texture(
                &texture_creator,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
                game.iris.cur_radius,
            )?;
            canvas.copy(&mask, None, None)?;
        }

        // Flash "Sauvegardé !" (F5)
        // Dans le rendu, remplacer le bloc flash actuel :
        if save_flash_timer > 0.0 {
            // Fondu : opaque pendant 1s, puis disparaît sur 0.5s
            let alpha = if save_flash_timer > 0.5 {
                220u8
            } else {
                (save_flash_timer / 0.5 * 220.0) as u8
            };

            // Utiliser set_alpha_mod sur la texture
            save_flash_tex.set_alpha_mod(alpha);
            canvas.copy(&save_flash_tex, None, Some(Rect::new(
                (WINDOW_WIDTH as i32 - save_flash_w as i32) / 2,
                WINDOW_HEIGHT as i32 - 60,
                save_flash_w,
                save_flash_h,
            )))?;
        }

        canvas.present();

        // Limitation FPS
        let elapsed = last_frame_time.elapsed().as_micros() as u64;
        if elapsed < FRAME_DURATION_MICROS {
            std::thread::sleep(Duration::from_micros(FRAME_DURATION_MICROS - elapsed));
        }
    }

    Ok(())
}
