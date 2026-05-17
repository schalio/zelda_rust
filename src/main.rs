// src/main.rs

mod camera;
mod tilemap;
mod player;
mod enemy;
mod enemy_type;
pub mod tile_properties;
pub mod map_loader;
pub mod combat;
pub mod hud;
pub mod audio;
pub mod objects;
pub mod transition;
pub mod npc;

use audio::AudioManager;
use camera::Camera;
use combat::{resolve_bush_cut, resolve_enemy_contact, resolve_object_contact, resolve_player_attack};
use enemy::Enemy;
use hud::{Hud, FONT_SIZE};
use map_loader::{load_tmx, MapFile, ObjectKind, SpawnKind, KeyKind};
use npc::{find_npc_in_front, render_npcs, update_npcs};
use objects::{render_objects, resolve_chest_collision};
use player::{DeathState, Player};
use tile_properties::TileTable;
use tilemap::Tilemap;
use transition::{create_iris_texture, IrisTransition};

use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::keyboard::Keycode;
use sdl2::mixer;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::collections::HashMap;
use std::time::{Duration, Instant};

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const TARGET_FPS: u64 = 60;
const FRAME_DURATION_MICROS: u64 = 1_000_000 / TARGET_FPS;


fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;

    let ttf_context = sdl2::ttf::init()
        .map_err(|e| format!("Erreur init TTF: {e}"))?;

    let mut audio = AudioManager::new()?;
    audio.play_music("assets/music/dungeon.ogg")?;

    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Zelda-like Rust", WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();

    // --- Chargement des textures ---
    let spritesheet = texture_creator.load_texture("assets/sprites/player.png")?;
    let enemy_sheet = texture_creator.load_texture("assets/sprites/enemies.png")?;
    let slash_sheet = texture_creator.load_texture("assets/sprites/sword_slash.png")?;
    let vanish_sheet = texture_creator.load_texture("assets/sprites/vanish.png")?;
    let objects_sheet = texture_creator.load_texture("assets/sprites/objects.png")?;
    let npc_texture = texture_creator.load_texture("assets/sprites/npcs.png")?;

    // --- Chargement Tiled ---
    let (mut map_file, mut tile_table, tileset_png) = load_map("zelda_test")?;
    let mut tileset  = texture_creator.load_texture(&tileset_png)?;
    let mut objects = map_file.objects.clone();
    let mut npcs = map_file.npcs.clone();
    let player_spawn = map_file.spawn_points.iter()
        .find(|sp| matches!(sp.kind, SpawnKind::Player))
        .expect("❌ Aucun spawn 'player' dans la map !");

    let mut player = Player::new(player_spawn.x, player_spawn.y);

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

    let mut event_pump = sdl_context.event_pump()?;
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

    let mut collected_objects: HashMap<String, Vec<(i32, i32)>> = HashMap::new();

    let mut current_map_name = "zelda_test".to_string();

    let mut flash_timer: f32 = 0.0;
    const FLASH_DURATION: f32 = 0.3;

    let mut iris = IrisTransition::new();

    // let mut active_dialogue: Option<String> = None;

    let mut dialogue_pages: Vec<String> = Vec::new();
    let mut dialogue_page: usize = 0;
    let mut dialogue_close_cooldown: u32 = 0;

    let mut interact_pressed_last_frame = false;

    'game_loop: loop {
        let now = Instant::now();
        let dt = now.duration_since(last_frame_time).as_secs_f32();
        flash_timer = (flash_timer - dt).max(0.0);
        last_frame_time = now;

        // --- Événements ---
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'game_loop,
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'game_loop,
                Event::KeyDown { keycode: Some(Keycode::M), .. } => {
                    if mixer::Music::is_paused() {
                        audio.resume_music();
                    } else {
                        audio.pause_music();
                    }
                }

                _ => {}
            }
        }

        // --- DÉCLENCHEMENT de la transition ---
        // let _map_to_load = iris.update(dt);

        if !iris.is_active() && dialogue_pages.is_empty() {
            if dialogue_close_cooldown > 0 {
                dialogue_close_cooldown -= 1; // ← décompte ici
            } else {
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
                            iris.start(target_map_val, target_entry_val);
                            break;
                        }
                        Some(key_kind) => {
                            if objects[i].collected || player.has_key(key_kind) {
                                if !objects[i].collected {
                                    // Première ouverture : consommer la clé et marquer
                                    player.use_key(key_kind);
                                    objects[i].collected = true;
                                }
                                iris.start(target_map_val, target_entry_val);
                            } else {
                                let msg = match key_kind {
                                    KeyKind::Basic  => "Cette porte est verrouillée.\nIl te faut une clé.",
                                    KeyKind::Silver => "Cette porte requiert une clé d'argent.",
                                    KeyKind::Gold   => "Cette porte requiert une clé d'or.",
                                    KeyKind::Boss   => "Cette porte mène au boss.\nIl te faut la clé du donjon.",
                                };
                                dialogue_pages = split_dialogue(msg);
                                dialogue_page = 0;

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
        if let Some((target_map, target_entry)) = iris.update(dt) {
            save_collected(&objects, &current_map_name, &mut collected_objects);

            let (new_map, new_table, new_png) = load_map(&target_map)?;

            dialogue_pages.clear();
            dialogue_page = 0;

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
            apply_collected(&mut objects, &target_map, &collected_objects);
            current_map_name = target_map;

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

        let interact_just_pressed = interact_pressed && !interact_pressed_last_frame;
        interact_pressed_last_frame = interact_pressed;

        update_npcs(&mut npcs, dt, player.x, player.y, ground_layer, &tile_table);

        if !iris.is_active() && dialogue_pages.is_empty() {
            let player_events = player.update(dt, &kb, &ground_layer, &tile_table, &npcs);
            player.clamp_to_map(ground_layer.pixel_width(), ground_layer.pixel_height());
            if player_events.sword_swing {
                audio.play_sword();
            }
        }

        if interact_just_pressed {
            if !dialogue_pages.is_empty() {
                // Avancer à la page suivante ou fermer
                dialogue_page += 1;
                if dialogue_page >= dialogue_pages.len() {
                    dialogue_pages.clear();
                    dialogue_page = 0;
                    dialogue_close_cooldown = 30;
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
                if npcs[npc_index].patrol_target.is_none() {
                    dialogue_pages = split_dialogue(&npcs[npc_index].dialogue);
                    dialogue_page = 0;
                }
            } else {
                // Panneau
                for obj in &objects {
                    if let ObjectKind::Sign { text } = &obj.kind {
                        if is_in_front_of_player(player.x, player.y, player.direction, obj.x, obj.y) {
                            dialogue_pages = split_dialogue(text);
                            dialogue_page = 0;
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
                flash_timer = FLASH_DURATION;
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
        if flash_timer > 0.0 {
            let alpha = ((flash_timer / FLASH_DURATION) * 180.0) as u8;
            canvas.set_draw_color(Color::RGBA(255, 255, 255, alpha));
            canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
            canvas.fill_rect(Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT))?;
            canvas.set_blend_mode(sdl2::render::BlendMode::None);
        }

        // 4. HUD — toujours en dernier, par-dessus tout
        hud.render(&mut canvas, player.hp, player.max_hp, player.rubies, player.keys_basic as i32)?;

        if !dialogue_pages.is_empty() {
            let is_last = dialogue_page >= dialogue_pages.len() - 1;
            render_dialogue_box(
                &mut canvas, &texture_creator, &font,
                &dialogue_pages[dialogue_page],
                is_last,
            )?;
        }

        if iris.is_active() {
            let mask = create_iris_texture(
                &texture_creator,
                WINDOW_WIDTH,
                WINDOW_HEIGHT,
                iris.cur_radius,
            )?;
            canvas.copy(&mask, None, None)?;
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

fn is_in_front_of_player(
    px: f32, py: f32,
    dir: player::Direction,
    obj_x: f32, obj_y: f32,
) -> bool {
    use crate::tilemap::TILE_DRAW_SIZE;
    let reach = TILE_DRAW_SIZE as f32 * 1.2;

    // Centrer le panneau (coin haut-gauche → centre)
    let ox = obj_x + TILE_DRAW_SIZE as f32 * 0.5;
    let oy = obj_y + TILE_DRAW_SIZE as f32 * 0.5;

    let (dx, dy) = (ox - px, oy - py);
    match dir {
        player::Direction::Up    => dy < 0.0 && dy.abs() < reach && dx.abs() < reach,
        player::Direction::Down  => dy > 0.0 && dy.abs() < reach && dx.abs() < reach,
        player::Direction::Left  => dx < 0.0 && dx.abs() < reach && dy.abs() < reach,
        player::Direction::Right => dx > 0.0 && dx.abs() < reach && dy.abs() < reach,
    }
}

fn separate_enemies(enemies: &mut [Enemy], tilemap: &Tilemap, table: &TileTable) {
    for i in 0..enemies.len() {
        for j in (i + 1)..enemies.len() {
            if !enemies[i].is_alive || !enemies[j].is_alive {
                continue;
            }

            let dx = enemies[j].x - enemies[i].x;
            let dy = enemies[j].y - enemies[i].y;
            let dist_sq = dx * dx + dy * dy;

            let r1 = enemies[i].separation_radius();
            let r2 = enemies[j].separation_radius();
            let min_dist = r1 + r2;
            let min_dist_sq = min_dist * min_dist;

            if dist_sq < min_dist_sq {
                // Cas rare : deux ennemis exactement au même point
                if dist_sq < 0.0001 {
                    enemies[i].push_by(-1.0, 0.0, tilemap, table);
                    enemies[j].push_by( 1.0, 0.0, tilemap, table);
                    continue;
                }

                let dist = dist_sq.sqrt();
                let overlap = min_dist - dist;

                let nx = dx / dist;
                let ny = dy / dist;

                let push_x = nx * overlap * 0.5;
                let push_y = ny * overlap * 0.5;

                enemies[i].push_by(-push_x, -push_y, tilemap, table);
                enemies[j].push_by( push_x,  push_y, tilemap, table);
            }
        }
    }
}

/// Lit le chemin du PNG depuis le fichier TSX.
fn get_png_path_from_tsx(tsx_path: &str) -> Result<String, String> {
    let content = std::fs::read_to_string(tsx_path)
        .map_err(|e| format!("Impossible de lire '{tsx_path}': {e}"))?;

    let doc = roxmltree::Document::parse(&content)
        .map_err(|e| format!("Erreur XML: {e}"))?;

    let source = doc.root_element()
        .children()
        .find(|n| n.has_tag_name("image"))
        .and_then(|n| n.attribute("source"))
        .ok_or("Attribut 'source' manquant dans <image> du TSX")?;

    // Résoudre le chemin relatif au TSX
    let tsx_dir = std::path::Path::new(tsx_path)
        .parent()
        .unwrap_or(std::path::Path::new("."));

    Ok(tsx_dir.join(source).to_string_lossy().into_owned())
}

fn load_map(name: &str) -> Result<(MapFile, TileTable, String), String> {
    let path = format!("assets/maps/{name}.tmx");
    let map_file   = load_tmx(&path)?;
    let tile_table = TileTable::from_tsx(&map_file.tileset_path)?;
    let png_path   = get_png_path_from_tsx(&map_file.tileset_path)?;
    Ok((map_file, tile_table, png_path.to_string()))
}

fn save_collected(
    objects: &[map_loader::MapObject],
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

fn apply_collected(
    objects: &mut Vec<map_loader::MapObject>,
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

fn game_over_screen(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    texture_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    ttf_context: &sdl2::ttf::Sdl2TtfContext,
    event_pump: &mut sdl2::EventPump,
) -> Result<(), String> {

    let font_large = ttf_context
        .load_font("assets/fonts/zelda.ttf", 32)
        .map_err(|e| e.to_string())?;

    let font_small = ttf_context
        .load_font("assets/fonts/zelda.ttf", 12)
        .map_err(|e| e.to_string())?;

    // Pré-rendu des textes
    let title_surf = font_large.render("GAME OVER")
        .blended(Color::RGB(200, 40, 40))
        .map_err(|e| e.to_string())?;

    let sub_surf = font_small.render("Appuyez sur une touche...")
        .blended(Color::RGB(180, 180, 180))
        .map_err(|e| e.to_string())?;

    let mut title_tex = texture_creator.create_texture_from_surface(title_surf)
        .map_err(|e| e.to_string())?;

    let mut sub_tex = texture_creator.create_texture_from_surface(sub_surf)
        .map_err(|e| e.to_string())?;

    let mut img_tex = texture_creator
        .load_texture("assets/sprites/game_over.png")
        .map_err(|e| e.to_string())?;

    let tw = title_tex.query().width;
    let th = title_tex.query().height;

    let sw = sub_tex.query().width;
    let sh = sub_tex.query().height;

    let iw = img_tex.query().width;
    let ih = img_tex.query().height;

    // Centrage de l'image (au-dessus du titre)
    let img_x = (WINDOW_WIDTH  as i32 - iw as i32) / 2;
    let img_y = (WINDOW_HEIGHT as i32 / 2) - ih as i32 - th as i32 - 30;

    // Fondu depuis noir
    let mut alpha: f32 = 0.0;

    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
    title_tex.set_blend_mode(sdl2::render::BlendMode::Blend);
    sub_tex.set_blend_mode(sdl2::render::BlendMode::Blend);
    img_tex.set_blend_mode(sdl2::render::BlendMode::Blend);

    'go_loop: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'go_loop,
                Event::KeyDown { .. } | Event::MouseButtonDown { .. } => {
                    if alpha >= 250.0 { break 'go_loop; }
                }
                _ => {}
            }
        }

        alpha = (alpha + 3.0).min(255.0);
        let a = alpha as u8;

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        // Image en fondu
        img_tex.set_alpha_mod(a);
        canvas.copy(&img_tex, None, Some(Rect::new(img_x, img_y, iw, ih)))?;

        // Titre centré
        title_tex.set_alpha_mod(a);
        let tx = (WINDOW_WIDTH  as i32 - tw as i32) / 2;
        let ty = (WINDOW_HEIGHT as i32 / 2) - th as i32 - 10;
        canvas.copy(&title_tex, None, Some(Rect::new(tx, ty, tw, th)))?;

        // Sous-titre centré
        if alpha >= 255.0 {
            sub_tex.set_alpha_mod(255);
            let sx = (WINDOW_WIDTH  as i32 - sw as i32) / 2;
            let sy = ty + th as i32 + 20;
            canvas.copy(&sub_tex, None, Some(Rect::new(sx, sy, sw, sh)))?;
        }

        canvas.present();
        std::thread::sleep(Duration::from_millis(16));
    }

    Ok(())
}

fn render_dialogue_box(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    texture_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    font: &sdl2::ttf::Font,
    text: &str,
    is_last: bool,
) -> Result<(), String> {
    let box_x = 32;
    let box_h = 120;
    let box_y = WINDOW_HEIGHT as i32 - box_h - 24;
    let box_w = WINDOW_WIDTH - 64;

    canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
    canvas.set_draw_color(Color::RGBA(0, 0, 0, 220));
    canvas.fill_rect(Rect::new(box_x, box_y, box_w, box_h as u32))?;

    canvas.set_draw_color(Color::RGB(255, 255, 255));
    canvas.draw_rect(Rect::new(box_x, box_y, box_w, box_h as u32))?;

    let surface = font
        .render(text)
        .blended_wrapped(Color::RGB(255, 255, 255), box_w - 24)
        .map_err(|e| e.to_string())?;

    let texture = texture_creator
        .create_texture_from_surface(&surface)
        .map_err(|e| e.to_string())?;

    let query = texture.query();
    let text_x = box_x + 12;
    let text_y = box_y + 12;

    canvas.copy(
        &texture,
        None,
        Some(Rect::new(text_x, text_y, query.width, query.height)),
    )?;

    // Indicateur en bas à droite
    let indicator = if is_last { "[ E ] Fermer" } else { "[ E ] Suite ▶" };
    let ind_surf = font
        .render(indicator)
        .blended(Color::RGB(180, 180, 180))
        .map_err(|e| e.to_string())?;
    let ind_tex = texture_creator
        .create_texture_from_surface(&ind_surf)
        .map_err(|e| e.to_string())?;
    let iw = ind_tex.query().width;
    let ih = ind_tex.query().height;
    canvas.copy(
        &ind_tex,
        None,
        Some(Rect::new(
            box_x + box_w as i32 - iw as i32 - 12,
            box_y + box_h - ih as i32 - 8,
            iw, ih,
        )),
    )?;

    Ok(())
}

fn split_dialogue(text: &str) -> Vec<String> {
    // Normalise : remplace les \n littéraux par de vrais sauts de ligne
    let normalized = text.replace("\\n", "\n");

    // Sépare sur double saut de ligne
    normalized.split("\n\n")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}