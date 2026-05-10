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

use audio::AudioManager;
use camera::Camera;
use combat::{check_transition, resolve_enemy_contact, resolve_object_contact, resolve_player_attack};
use enemy::Enemy;
use hud::{Hud, FONT_SIZE};
use map_loader::{load_tmx, MapFile, SpawnKind, ObjectKind};
use objects::{render_objects, resolve_chest_collision};
use player::{DeathState, Player};
use tile_properties::TileTable;
use tilemap::Tilemap;
use transition::{IrisTransition, create_iris_texture};

use sdl2::event::Event;
use sdl2::image::LoadTexture;
use sdl2::keyboard::Keycode;
use sdl2::mixer;
use sdl2::pixels::Color;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use sdl2::rect::Rect;

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
    // (reprendre la création du tileset du chapitre 2 ici)
    // let tileset = create_test_tileset(&texture_creator)?;
    // let tileset = texture_creator.load_texture("assets/tiles/tileset.png")?;
    // let spritesheet = create_test_spritesheet(&texture_creator)?;
    let spritesheet = texture_creator.load_texture("assets/sprites/player.png")?;
    //let enemy_sheet = create_enemy_spritesheet(&texture_creator)?;
    let enemy_sheet = texture_creator.load_texture("assets/sprites/enemies.png")?;
    let slash_sheet = texture_creator.load_texture("assets/sprites/sword_slash.png")?;
    let vanish_sheet = texture_creator.load_texture("assets/sprites/vanish.png")?;
    let objects_sheet = texture_creator.load_texture("assets/sprites/objects.png")?;
/*
    // --- Carte ---
    #[rustfmt::skip]
    let map_data: Vec<Vec<u8>> = vec![
        vec![1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
        vec![1,0,0,0,0,0,3,3,0,0,0,0,0,0,0,3,3,0,0,1],
        vec![1,0,0,0,0,0,3,3,0,0,0,0,0,0,0,3,3,0,0,1],
        vec![1,0,0,1,1,0,0,0,0,0,1,1,0,0,0,0,0,0,0,1],
        vec![1,0,0,1,1,0,0,0,0,0,1,1,0,0,0,0,0,0,0,1],
        vec![1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
        vec![1,0,0,0,0,0,0,2,2,0,0,0,0,0,0,0,0,0,0,1],
        vec![1,0,0,0,0,0,0,2,2,0,0,0,0,0,0,0,0,0,0,1],
        vec![1,0,0,0,0,0,0,0,0,0,0,0,0,1,1,0,0,0,0,1],
        vec![1,0,0,0,0,0,0,0,0,0,0,0,0,1,1,0,0,0,0,1],
        vec![1,0,3,3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
        vec![1,0,3,3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
        vec![1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
        vec![1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1],
        vec![1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],
    ];
    let tilemap = Tilemap::new(map_data);
*/
    // --- Chargement Tiled ---
    //let map_file   = load_tmx("assets/maps/zelda_test.tmx")?;
    let (mut map_file, mut tile_table, tileset_png) = load_map("zelda_test")?;
    let mut tileset  = texture_creator.load_texture(&tileset_png)?;
    // println!("{}", map_file.tileset_path);
    // let tile_table = TileTable::from_tsx(&map_file.tileset_path)?;
    // let mut tileset    = texture_creator.load_texture(get_png_path_from_tsx(&map_file.tileset_path)?)?;

    // Les couches : 0 = sol (collisions), 1+ = décor au-dessus
    // On utilise la première couche pour les collisions
    // let ground_layer = &map_file.layers[0].tilemap;
    let mut objects = map_file.objects.clone();
/*
    // --- Joueur ---
    let mut player = Player::new(
        ground_layer.pixel_width() / 2.0,
        ground_layer.pixel_height() / 2.0,
    );
    // Spawner plusieurs ennemis à des positions précises de la carte
    let mut enemies: Vec<Enemy> = vec![
        Enemy::new(3.0 * TILE_DRAW_SIZE as f32, 5.0 * TILE_DRAW_SIZE as f32, EnemyKind::Slime),
        Enemy::new(11.0 * TILE_DRAW_SIZE as f32, 7.0 * TILE_DRAW_SIZE as f32, EnemyKind::Goblin),
        Enemy::new(15.0 * TILE_DRAW_SIZE as f32, 3.0 * TILE_DRAW_SIZE as f32, EnemyKind::Knight),
        Enemy::new(5.0 * TILE_DRAW_SIZE as f32, 12.0 * TILE_DRAW_SIZE as f32, EnemyKind::Slime),
    ];
*/
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

    // let mut fade = FadeTransition::new();
    let mut iris = IrisTransition::new();


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
/*
        if !fade.is_active() {
            if let Some((target_map, target_entry)) = check_transition(&player, &objects) {
                fade.start(target_map, target_entry);
            }
        }
*/
        // --- DÉCLENCHEMENT de la transition ---
        let _map_to_load = iris.update(dt);

        if !iris.is_active() {
            if let Some((target_map, target_entry)) = check_transition(&player, &objects) {
                iris.start(target_map, target_entry);
            }
        }

/*
        if let Some((target_map, target_entry)) = fade.update(dt) {
            // === RECHARGEMENT DE LA MAP (déplacé ici depuis l'ancien if let) ===
            save_collected(&objects, &current_map_name, &mut collected_objects);

            let (new_map, new_table, new_png) = load_map(&target_map)?;

            let entry = new_map.spawn_points.iter()
                .find(|sp| sp.name == target_entry)
                .or_else(|| new_map.spawn_points.iter()
                    .find(|sp| matches!(sp.kind, SpawnKind::Player)))
                .expect(&format!("❌ Spawn '{target_entry}' introuvable"));

            player.x = entry.x;
            player.y = entry.y;

            tileset  = texture_creator.load_texture(&new_png)?;
            objects  = new_map.objects.clone();

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
*/

        // --- RECHARGEMENT au moment où l'écran est noir ---
        if let Some((target_map, target_entry)) = iris.update(dt) {
            save_collected(&objects, &current_map_name, &mut collected_objects);

            let (new_map, new_table, new_png) = load_map(&target_map)?;

            let entry = new_map.spawn_points.iter()
                .find(|sp| sp.name == target_entry)
                .or_else(|| new_map.spawn_points.iter()
                    .find(|sp| matches!(sp.kind, SpawnKind::Player)))
                .expect(&format!("❌ Spawn '{target_entry}' introuvable"));

            player.x = entry.x;
            player.y = entry.y;

            tileset  = texture_creator.load_texture(&new_png)?;
            objects  = new_map.objects.clone();
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


        /*
        if let Some((target_map, target_entry)) = check_transition(&player, &objects) {

            // println!("→ transition vers {target_map}, current={current_map_name}");

            let target_map   = target_map.clone();
            let target_entry = target_entry.clone();

            save_collected(&objects, &current_map_name, &mut collected_objects);

            let (new_map, new_table, new_png) = load_map(&target_map)?;

            // ← ici, remplacez le find précédent par :
            let entry = new_map.spawn_points.iter()
                .find(|sp| sp.name == target_entry)
                .or_else(|| new_map.spawn_points.iter()
                    .find(|sp| matches!(sp.kind, SpawnKind::Player)))
                .expect(&format!("❌ Spawn '{target_entry}' introuvable"));

            player.x = entry.x;
            player.y = entry.y;

            tileset    = texture_creator.load_texture(&new_png)?;
            objects    = new_map.objects.clone();

            apply_collected(&mut objects, &target_map, &collected_objects);

            current_map_name = target_map;

            enemies    = new_map.spawn_points.iter()
                .filter_map(|sp| {
                    if let SpawnKind::Enemy(kind) = sp.kind {
                        Some(Enemy::new(sp.x, sp.y, kind))
                    } else { None }
                })
                .collect();
            tile_table = new_table;
            map_file   = new_map;
        }
*/
        let ground_layer = &map_file.layers[0].tilemap;

        // --- Mise à jour ---
        let kb = event_pump.keyboard_state();

//        if !fade.is_active() {
        if !iris.is_active() {
            let player_events = player.update(dt, &kb, &ground_layer, &tile_table);
            player.clamp_to_map(ground_layer.pixel_width(), ground_layer.pixel_height());
            if player_events.sword_swing { audio.play_sword();}
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
/*
            for enemy in enemies.iter() {
                if enemy.collides_with_player(player.x, player.y, player::HITBOX_HALF) {
                    player.take_hit(enemy.x, enemy.y, enemy.contact_damage());
                }
            }
*/
            separate_enemies(&mut enemies, &ground_layer, &tile_table);
            resolve_enemy_contact(&mut player, &mut enemies, &audio);
            resolve_player_attack(&player, &mut enemies, &audio);
            if resolve_object_contact(&mut player, &mut objects, &audio) {
                flash_timer = FLASH_DURATION;
            }

            resolve_chest_collision(&mut player, &objects);



            // Nettoyer les ennemis morts dont l'animation est terminée
            enemies.retain(|e| e.is_alive || e.death_timer > 0.0);
        }
        // Vérifier game over
        // main.rs — remplacez le bloc game over par :
        // Remplacez le bloc DeathState::Done par :
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
/*
        for event in event_pump.poll_iter() {
            match event {
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
*/
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

        // 1.1 Objets au sol  ← nouveau
        render_objects(&objects, &mut canvas, &objects_sheet, &camera)?;

        // 2. Ennemis
        for enemy in enemies.iter() {
            enemy.render(&mut canvas, &enemy_sheet, &vanish_sheet, &camera)?;
            // enemy.render_debug(&mut canvas, &camera)?; // décommenter pour debug
        }

        // 3. Joueur
        player.render(&mut canvas, &spritesheet, &slash_sheet, &vanish_sheet, &camera)?;
        // player.render_hitbox(&mut canvas, &camera)?;

        // Flash de collecte
        if flash_timer > 0.0 {
            let alpha = ((flash_timer / FLASH_DURATION) * 180.0) as u8;
            canvas.set_draw_color(Color::RGBA(255, 255, 255, alpha));
            canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
            canvas.fill_rect(Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT))?;
            canvas.set_blend_mode(sdl2::render::BlendMode::None);
        }

        // 4. HUD — toujours en dernier, par-dessus tout
        hud.render(&mut canvas, player.hp, player.max_hp, player.rubies, player.keys)?;

/*
        if fade.is_active() && fade.alpha > 0 {
            canvas.set_blend_mode(sdl2::render::BlendMode::Blend);
            canvas.set_draw_color(sdl2::pixels::Color::RGBA(0, 0, 0, fade.alpha));
            canvas.fill_rect(sdl2::rect::Rect::new(
                0, 0, WINDOW_WIDTH, WINDOW_HEIGHT
            ))?;
            canvas.set_blend_mode(sdl2::render::BlendMode::None);
        }
*/

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

/*
/// Crée un spritesheet de test : 4 colonnes (frames) × 4 lignes (directions)
/// Chaque direction a une couleur dominante + une flèche dessinée en pixels.
fn create_test_spritesheet(
    texture_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
) -> Result<sdl2::render::Texture<'_>, String> {
    use sdl2::pixels::PixelFormatEnum;

    const W: u32 = 16 * 4;  // 4 frames
    const H: u32 = 16 * 4;  // 4 directions

    let mut tex = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGBA8888, W, H)
        .map_err(|e| e.to_string())?;

    // Couleur de base par direction (Bas, Gauche, Droite, Haut)
    let dir_colors: [(u8, u8, u8); 4] = [
        (100, 180, 100), // Bas : vert
        (100, 100, 200), // Gauche : bleu
        (200, 150, 80),  // Droite : orange
        (200, 100, 100), // Haut : rouge
    ];

    tex.with_lock(None, |buf: &mut [u8], pitch: usize| {
        for dir in 0..4usize {
            let (br, bg, bb) = dir_colors[dir];
            for frame in 0..4usize {
                // Légère variation de luminosité entre frames (effet animation)
                let variation = (frame as u8) * 15;
                let r = br.saturating_add(variation);
                let g = bg.saturating_add(variation);
                let b = bb.saturating_add(variation);

                for py in 0..16usize {
                    for px in 0..16usize {
                        let world_x = frame * 16 + px;
                        let world_y = dir * 16 + py;
                        let offset = world_y * pitch + world_x * 4;

                        // Corps du personnage (carré plein avec contour)
                        let is_border = px == 0 || px == 15 || py == 0 || py == 15;
                        // Tête (carré 6×6 centré en haut)
                        let is_head = px >= 5 && px <= 10 && py >= 2 && py <= 7;

                        let (pr, pg, pb) = if is_border {
                            (r / 3, g / 3, b / 3)
                        } else if is_head {
                            (220, 190, 140) // couleur peau
                        } else {
                            (r, g, b)
                        };

                        buf[offset]     = 255;
                        buf[offset + 1] = pb;
                        buf[offset + 2] = pg;
                        buf[offset + 3] = pr;
                    }
                }
            }
        }
    })?;

    Ok(tex)
}

/// Crée un tileset de test en mémoire : 4 tuiles de 16×16 px côte à côte (64×16 px total)
/// 0=herbe (vert), 1=rocher (gris), 2=eau (bleu), 3=arbre (vert foncé)
fn create_test_tileset(
    texture_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
) -> Result<sdl2::render::Texture<'_>, String> {
    use sdl2::pixels::PixelFormatEnum;
    use crate::tilemap::TILE_SIZE;

    const NB_TILES: u32 = 4;

    let mut tex = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGBA8888, TILE_SIZE * NB_TILES, TILE_SIZE)
        .map_err(|e| e.to_string())?;

    // Couleur de base de chaque tuile (R, G, B)
    let tile_colors: [(u8, u8, u8); 4] = [
        (80,  160,  80), // 0 : Herbe (vert moyen)
        (120, 120, 120), // 1 : Rocher (gris)
        (60,  120, 200), // 2 : Eau (bleu)
        (30,   90,  30), // 3 : Arbre (vert foncé)
    ];

    tex.with_lock(None, |buf: &mut [u8], pitch: usize| {
        for tile_idx in 0..NB_TILES as usize {
            let (r, g, b) = tile_colors[tile_idx];

            for py in 0..TILE_SIZE as usize {
                for px in 0..TILE_SIZE as usize {
                    // Contour sombre sur le bord de chaque tuile
                    // pour visualiser la grille pendant le développement
                    let is_border = px == 0 || px == TILE_SIZE as usize - 1
                        || py == 0 || py == TILE_SIZE as usize - 1;

                    let (pr, pg, pb) = if is_border {
                        (r / 2, g / 2, b / 2) // assombrir le bord
                    } else {
                        (r, g, b)
                    };

                    // Position du pixel dans le buffer linéaire
                    // Format RGBA8888 : 4 octets par pixel, ordre A-B-G-R en mémoire
                    let world_x = tile_idx * TILE_SIZE as usize + px;
                    let offset  = py * pitch + world_x * 4;

                    buf[offset]     = 255; // A (alpha, opaque)
                    buf[offset + 1] = pb;  // B
                    buf[offset + 2] = pg;  // G
                    buf[offset + 3] = pr;  // R
                }
            }
        }
    })?;

    Ok(tex)
}

/// Spritesheet ennemis : 3 types × 4 directions × 4 frames = 16×48 px source
/// Organisé en lignes : Slime(vert), Goblin(orange), Knight(bleu-gris)
/// Chaque type occupe 4 lignes (une par direction)
fn create_enemy_spritesheet(
    texture_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
) -> Result<sdl2::render::Texture<'_>, String> {
    use sdl2::pixels::PixelFormatEnum;

    const SPRITE_W: u32 = 16;
    const SPRITE_H: u32 = 16;
    const NB_TYPES: u32 = 3;
    const NB_DIRS: u32 = 4;
    const NB_FRAMES: u32 = 4;

    let tex_w = SPRITE_W * NB_FRAMES;
    let tex_h = SPRITE_H * NB_TYPES * NB_DIRS;

    let mut tex = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGBA8888, tex_w, tex_h)
        .map_err(|e| e.to_string())?;

    // Couleur par type (Slime, Goblin, Knight)
    let type_colors: [(u8, u8, u8); 3] = [
        (80,  200,  80),  // Slime  : vert
        (200, 140,  40),  // Goblin : orange
        (140, 140, 200),  // Knight : bleu-gris
    ];

    tex.with_lock(None, |buf: &mut [u8], pitch: usize| {
        for enemy_type in 0..NB_TYPES as usize {
            let (r, g, b) = type_colors[enemy_type];
            for dir in 0..NB_DIRS as usize {
                for frame in 0..NB_FRAMES as usize {
                    let variation = (frame as u8) * 12;
                    let fr = r.saturating_add(variation);
                    let fg = g.saturating_add(variation);
                    let fb = b.saturating_add(variation);

                    for py in 0..SPRITE_H as usize {
                        for px in 0..SPRITE_W as usize {
                            let world_x = frame * SPRITE_W as usize + px;
                            let world_y = (enemy_type * NB_DIRS as usize + dir)
                                * SPRITE_H as usize + py;
                            let offset = world_y * pitch + world_x * 4;

                            let is_border = px == 0 || px == 15 || py == 0 || py == 15;
                            let (pr, pg, pb) = if is_border {
                                (fr / 3, fg / 3, fb / 3)
                            } else {
                                (fr, fg, fb)
                            };

                            buf[offset]     = 255;
                            buf[offset + 1] = pb;
                            buf[offset + 2] = pg;
                            buf[offset + 3] = pr;
                        }
                    }
                }
            }
        }
    })?;

    Ok(tex)
}
*/

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
            ObjectKind::HeartPiece | ObjectKind::Chest { .. }
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
            if matches!(obj.kind, ObjectKind::HeartPiece | ObjectKind::Chest { .. }) {
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