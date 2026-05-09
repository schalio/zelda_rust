// src/audio.rs
// Gestion centralisée de la musique et des effets sonores via SDL2_mixer.

use sdl2::mixer::{
    self, Chunk, InitFlag, Music, Sdl2MixerContext,
    DEFAULT_CHANNELS, AUDIO_S16LSB,
};

/// Fréquence audio standard
const AUDIO_FREQUENCY: i32  = 44_100;
/// Taille du buffer audio (plus petit = moins de latence)
const AUDIO_CHUNK_SIZE: i32 = 1_024;
/// Nombre de canaux simultanés pour les effets sonores
const MIXER_CHANNELS: i32   = 16;

/// Canaux dédiés par type de son (évite les chevauchements indésirables)
const CHANNEL_PLAYER_HIT:   i32 = 0;
pub const CHANNEL_PLAYER_DEATH: i32 = 1;
const CHANNEL_SWORD:        i32 = 2;
pub const CHANNEL_PICKUP: i32 = 3;   // rubis + cœur
pub const CHANNEL_CHEST:  i32 = 4;   // coffre
// Canaux 3-15 : libres pour les ennemis

/// Conteneur principal de tous les assets audio.
/// À créer une seule fois dans main() et passer en référence.
pub struct AudioManager {
    // Le contexte mixer doit rester en vie toute la durée du jeu
    _mixer_context: Sdl2MixerContext,
    current_music: Option<Music<'static>>,

    // Effets sonores
    pub sfx_sword:        Chunk,
    pub sfx_hit_enemy:    Chunk,
    pub sfx_hit_player:   Chunk,
    pub sfx_enemy_death:  Chunk,
    pub sfx_player_death: Chunk,
    pickup_ruby:  Option<Chunk>,
    pickup_heart: Option<Chunk>,
    chest_open:   Option<Chunk>,

    // Volume global (0-128)
    music_volume: i32,
    sfx_volume:   i32,
}

impl AudioManager {
    /// Initialise SDL2_mixer et charge tous les assets audio.
    pub fn new() -> Result<Self, String> {
        // Initialisation du sous-système mixer
        // InitFlag::OGG permet de lire les fichiers .ogg
        let _mixer_context = mixer::init(InitFlag::OGG)
            .map_err(|e| format!("Erreur init SDL2_mixer: {e}"))?;

        // Ouverture du périphérique audio
        mixer::open_audio(
            AUDIO_FREQUENCY,
            AUDIO_S16LSB,        // format 16 bits signé
            DEFAULT_CHANNELS,    // stéréo
            AUDIO_CHUNK_SIZE,
        ).map_err(|e| format!("Erreur open_audio: {e}"))?;

        // Allocation des canaux pour les effets simultanés
        mixer::allocate_channels(MIXER_CHANNELS);

        // Chargement des effets sonores
        let sfx_sword        = Self::load_chunk("assets/sounds/sword_slash.wav")?;
        let sfx_hit_enemy    = Self::load_chunk("assets/sounds/hit_enemy.wav")?;
        let sfx_hit_player   = Self::load_chunk("assets/sounds/hit_player.wav")?;
        let sfx_enemy_death  = Self::load_chunk("assets/sounds/enemy_death.wav")?;
        let sfx_player_death = Self::load_chunk("assets/sounds/player_death.wav")?;
        let pickup_ruby  = Chunk::from_file("assets/sounds/pickup_ruby.wav").ok();
        let pickup_heart = Chunk::from_file("assets/sounds/pickup_heart.wav").ok();
        let chest_open   = Chunk::from_file("assets/sounds/chest_open.wav").ok();
        
        Ok(AudioManager {
            _mixer_context,
            current_music: None,
            sfx_sword,
            sfx_hit_enemy,
            sfx_hit_player,
            sfx_enemy_death,
            sfx_player_death,
            pickup_ruby,
            pickup_heart,
            chest_open,
            music_volume: 64,   // 50% par défaut
            sfx_volume:   100,
        })
    }

    /// Charge un fichier .wav et retourne un Chunk SDL2.
    fn load_chunk(path: &str) -> Result<Chunk, String> {
        Chunk::from_file(path)
            .map_err(|e| format!("Impossible de charger '{path}': {e}"))
    }

    // --- Lecture de la musique ---

    /// Charge et joue la musique de fond en boucle infinie.
    pub fn play_music(&mut self, path: &str) -> Result<(), String> {
        let music = Music::from_file(path)
            .map_err(|e| format!("Impossible de charger la musique '{path}': {e}"))?;

        Music::set_volume(self.music_volume);

        // -1 = boucle infinie
        music.play(-1)
            .map_err(|e| format!("Erreur lecture musique: {e}"))?;

        // Important : on laisse SDL2_mixer gérer la musique en interne,
        // mais on doit garder l'objet Music en vie.
        // Solution : on le "oublie" via std::mem::forget — SDL2_mixer
        // conserve une référence interne tant que la musique joue.
        self.current_music = Some(unsafe {
            // SAFETY : le fichier reste valide tant que AudioManager existe
            std::mem::transmute(music)
        });

        Ok(())
    }

    pub fn pause_music(&self)  { Music::pause(); }
    pub fn resume_music(&self) { Music::resume(); }
    pub fn stop_music(&self)   { Music::halt(); }

    pub fn set_music_volume(&mut self, volume: i32) {
        self.music_volume = volume.clamp(0, 128);
        Music::set_volume(self.music_volume);
    }

    // --- Lecture des effets sonores ---

    /// Joue le son de l'attaque du joueur.
    /// Utilise un canal dédié pour ne pas couper d'autres sons.
    pub fn play_sword(&self) {
        let _ = mixer::Channel(CHANNEL_SWORD)
            .play(&self.sfx_sword, 0);
    }

    /// Joue le son d'un ennemi touché.
    pub fn play_hit_enemy(&self) {
        // Canal -1 = premier canal libre disponible
        let _ = mixer::Channel(-1)
            .play(&self.sfx_hit_enemy, 0);
    }

    /// Joue le son du joueur touché.
    pub fn play_hit_player(&self) {
        let _ = mixer::Channel(CHANNEL_PLAYER_HIT)
            .play(&self.sfx_hit_player, 0);
    }

    /// Joue le son de mort d'un ennemi.
    pub fn play_enemy_death(&self) {
        let _ = mixer::Channel(-1)
            .play(&self.sfx_enemy_death, 0);
    }

    /// Joue le son de mort du joueur + coupe la musique.
    // Dans play_player_death()
    pub fn play_player_death(&self) {
        // println!("→ play_player_death : arrêt musique + démarrage son");
        Music::halt();
        mixer::Channel(CHANNEL_PLAYER_DEATH).play(&self.sfx_player_death, 0).unwrap();
        // println!("→ canal {} playing={}", CHANNEL_PLAYER_DEATH, mixer::Channel(CHANNEL_PLAYER_DEATH as i32).is_playing());
    }

    pub fn play_pickup_ruby(&self) {
        if let Some(c) = &self.pickup_ruby {
            mixer::Channel(CHANNEL_PICKUP).play(c, 0).ok();
        }
    }

    pub fn play_pickup_heart(&self) {
        if let Some(c) = &self.pickup_heart {
            mixer::Channel(CHANNEL_PICKUP).play(c, 0).ok();
        }
    }

    pub fn play_chest_open(&self) {
        if let Some(c) = &self.chest_open {
            mixer::Channel(CHANNEL_CHEST).play(c, 0).ok();
        }
    }


    pub fn set_sfx_volume(&mut self, volume: i32) {
        self.sfx_volume = volume.clamp(0, 128);
        mixer::Channel::all().set_volume(self.sfx_volume);
    }
}