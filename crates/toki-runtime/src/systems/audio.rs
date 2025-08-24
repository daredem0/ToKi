use kira::{
    sound::static_sound::StaticSoundData, sound::static_sound::StaticSoundSettings, AudioManager,
    AudioManagerSettings,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use toki_core::{game::AudioEvent, EventHandler};

pub struct AudioSystem {
    manager: AudioManager,

    // Preloaded sounds - all SFX loaded at startup
    preloaded_sounds: HashMap<String, StaticSoundData>,

    // Music paths - discovered at startup, loaded on demand
    music_paths: HashMap<String, String>,
}

impl AudioSystem {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = AudioManager::new(AudioManagerSettings::default())?;
        let mut system = Self {
            manager,
            preloaded_sounds: HashMap::new(),
            music_paths: HashMap::new(),
        };

        system.scan_and_preload_sfx()?;
        system.scan_music_files()?;

        Ok(system)
    }

    fn scan_and_preload_sfx(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let sfx_dir = "assets/audio/sfx";

        if !Path::new(sfx_dir).exists() {
            tracing::warn!("SFX directory not found: {}", sfx_dir);
            return Ok(());
        }

        let entries = fs::read_dir(sfx_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if let Some(extension) = path.extension() {
                if extension == "ogg" {
                    if let Some(file_stem) = path.file_stem() {
                        if let Some(name) = file_stem.to_str() {
                            let path_str = path.to_string_lossy();
                            if let Err(e) = self.preload_sound(name, &path_str) {
                                tracing::warn!(
                                    "Failed to preload SFX '{}': 
  {}",
                                    name,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("Preloaded {} SFX files", self.preloaded_sounds.len());
        Ok(())
    }

    fn scan_music_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let music_dir = "assets/audio/music";

        if !Path::new(music_dir).exists() {
            tracing::warn!("Music directory not found: {}", music_dir);
            return Ok(());
        }

        let entries = fs::read_dir(music_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if let Some(extension) = path.extension() {
                if extension == "ogg" {
                    if let Some(file_stem) = path.file_stem() {
                        if let Some(name) = file_stem.to_str() {
                            let path_str = path.to_string_lossy().to_string();
                            self.music_paths.insert(name.to_string(), path_str);
                        }
                    }
                }
            }
        }

        tracing::info!("Discovered {} music files", self.music_paths.len());
        Ok(())
    }

    fn preload_sound(&mut self, name: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let sound_data = StaticSoundData::from_file(path)?;
        self.preloaded_sounds.insert(name.to_string(), sound_data);
        tracing::debug!("Preloaded SFX: {}", name);
        Ok(())
    }

    pub fn play_background_music(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Try on-demand music loading
        if let Some(path) = self.music_paths.get(name) {
            let sound_data = StaticSoundData::from_file(path)?
                .with_settings(StaticSoundSettings::new().loop_region(..));

            self.manager.play(sound_data)?;
            tracing::debug!("Played music on-demand: {}", name);
            return Ok(());
        }

        tracing::warn!("Audio file '{}' not found in SFX or music", name);
        Ok(())
    }

    pub fn play_sound(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Try preloaded SFX first (fast path)
        if let Some(sound_data) = self.preloaded_sounds.get(name) {
            self.manager.play(sound_data.clone())?;
            tracing::trace!("Played preloaded sound: {}", name);
            return Ok(());
        }

        // Try on-demand music loading
        if let Some(path) = self.music_paths.get(name) {
            let sound_data = StaticSoundData::from_file(path)?;
            self.manager.play(sound_data)?;
            tracing::debug!("Played music on-demand: {}", name);
            return Ok(());
        }

        tracing::warn!("Audio file '{}' not found in SFX or music", name);
        Ok(())
    }

    // Debug helper to list available sounds
    pub fn list_available_sounds(&self) {
        tracing::info!(
            "Available SFX: {:?}",
            self.preloaded_sounds.keys().collect::<Vec<_>>()
        );
        tracing::info!(
            "Available Music: {:?}",
            self.music_paths.keys().collect::<Vec<_>>()
        );
    }
}

impl std::fmt::Debug for AudioSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioSystem")
            .field("preloaded_sounds_count", &self.preloaded_sounds.len())
            .field("music_paths_count", &self.music_paths.len())
            .finish()
    }
}

impl EventHandler<AudioEvent> for AudioSystem {
    fn handle(&mut self, event: &AudioEvent) {
        match event {
            AudioEvent::PlayerWalk => {
                if let Err(e) = self.play_sound("sfx_jump") {
                    tracing::debug!("Failed to play footstep sound: {}", e);
                }
            }
            AudioEvent::PlayerCollision => {
                if let Err(e) = self.play_sound("sfx_coin") {
                    tracing::debug!("Failed to play collision sound: {}", e);
                }
            }
            AudioEvent::BackgroundMusic(name) => {
                if let Err(e) = self.play_background_music(name) {
                    tracing::warn!("Failed to play background music '{}': {}", name, e);
                }
            }
        }
    }
}
