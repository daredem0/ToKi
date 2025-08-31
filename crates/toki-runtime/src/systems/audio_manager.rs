use kira::{
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
    sound::streaming::{StreamingSoundData, StreamingSoundHandle, StreamingSoundSettings},
    AudioManager as KiraAudioManager, AudioManagerSettings, Tween,
    sound::FromFileError,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use toki_core::{game::AudioEvent, EventHandler};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackPolicy {
    /// Allow multiple instances of the same sound to play simultaneously
    Overlap,
    /// Stop any existing sounds in this channel before playing new one
    Exclusive,
    /// Ignore new sound requests if channel is already playing
    IgnoreIfPlaying,
}

#[derive(Debug)]
struct AudioChannel {
    policy: PlaybackPolicy,
    active_handles: Vec<StaticSoundHandle>,
    active_streaming_handles: Vec<StreamingSoundHandle<FromFileError>>,
    last_played: Option<Instant>,
    cooldown_duration: Option<Duration>,
}

pub struct AudioManager {
    manager: KiraAudioManager,

    // Preloaded sounds - all SFX loaded at startup
    preloaded_sounds: HashMap<String, StaticSoundData>,

    // Music paths - discovered at startup, loaded on demand
    music_paths: HashMap<String, String>,

    // Audio channels with policies
    channels: HashMap<String, AudioChannel>,
}

impl AudioManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = KiraAudioManager::new(AudioManagerSettings::default())?;
        let mut system = Self {
            manager,
            preloaded_sounds: HashMap::new(),
            music_paths: HashMap::new(),
            channels: HashMap::new(),
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
        tracing::trace!("Preloaded SFX: {}", name);
        Ok(())
    }

    /// Create or configure an audio channel with a specific playback policy
    pub fn set_channel_policy(&mut self, channel: &str, policy: PlaybackPolicy) {
        tracing::trace!("Setting channel '{}' policy to {:?}", channel, policy);
        self.channels.insert(channel.to_string(), AudioChannel {
            policy,
            active_handles: Vec::new(),
            active_streaming_handles: Vec::new(),
            last_played: None,
            cooldown_duration: None,
        });
    }

    /// Set a cooldown duration for a channel to prevent rapid-fire sounds
    pub fn set_channel_cooldown(&mut self, channel: &str, cooldown: Duration) {
        if let Some(channel_data) = self.channels.get_mut(channel) {
            channel_data.cooldown_duration = Some(cooldown);
            tracing::trace!("Set cooldown for channel '{}' to {:?}", channel, cooldown);
        } else {
            // Create channel with default policy and cooldown
            self.channels.insert(channel.to_string(), AudioChannel {
                policy: PlaybackPolicy::Overlap,
                active_handles: Vec::new(),
                active_streaming_handles: Vec::new(),
                last_played: None,
                cooldown_duration: Some(cooldown),
            });
            tracing::trace!("Created channel '{}' with cooldown {:?}", channel, cooldown);
        }
    }

    /// Stop all sounds in a specific channel
    pub fn stop_channel(&mut self, channel: &str) {
        if let Some(channel_data) = self.channels.get_mut(channel) {
            let total_stopped = channel_data.active_handles.len() + channel_data.active_streaming_handles.len();
            tracing::debug!("Stopping {} sounds in channel '{}'", total_stopped, channel);
            
            // Stop all static sound handles
            for handle in &mut channel_data.active_handles {
                handle.stop(Tween::default());
            }
            channel_data.active_handles.clear();

            // Stop all streaming handles
            for handle in &mut channel_data.active_streaming_handles {
                handle.stop(Tween::default());
            }
            channel_data.active_streaming_handles.clear();
        } else {
            tracing::trace!("Channel '{}' not found when trying to stop", channel);
        }
    }

    /// Clean up finished sound handles from all channels
    fn cleanup_finished_sounds(&mut self) {
        for (channel_name, channel) in &mut self.channels {
            let initial_static = channel.active_handles.len();
            let initial_streaming = channel.active_streaming_handles.len();
            
            // Trace: log states before cleanup
            if initial_static > 0 || initial_streaming > 0 {
                tracing::trace!("Channel '{}' before cleanup: {} static, {} streaming", 
                    channel_name, initial_static, initial_streaming);
                for (i, handle) in channel.active_handles.iter().enumerate() {
                    tracing::trace!("  Static handle {}: state={:?}", i, handle.state());
                }
                for (i, handle) in channel.active_streaming_handles.iter().enumerate() {
                    tracing::trace!("  Streaming handle {}: state={:?}", i, handle.state());
                }
            }
            
            channel.active_handles.retain(|handle| {
                handle.state() != kira::sound::PlaybackState::Stopped
            });
            channel.active_streaming_handles.retain(|handle| {
                handle.state() != kira::sound::PlaybackState::Stopped
            });
            
            let removed_static = initial_static - channel.active_handles.len();
            let removed_streaming = initial_streaming - channel.active_streaming_handles.len();
            let total_removed = removed_static + removed_streaming;
            
            if total_removed > 0 {
                tracing::trace!("Cleaned up {} finished sounds from channel '{}' (static: {}, streaming: {})",
                    total_removed, channel_name, removed_static, removed_streaming);
            }
        }
    }

    /// Play sound in a specific channel with channel policy enforcement
    pub fn play_sound_in_channel(&mut self, channel: &str, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.cleanup_finished_sounds();

        // Get or create channel with default Overlap policy
        if !self.channels.contains_key(channel) {
            tracing::trace!("Creating new channel '{}' with default Overlap policy", channel);
            self.set_channel_policy(channel, PlaybackPolicy::Overlap);
        }

        let channel_data = self.channels.get_mut(channel).unwrap();
        let active_count = channel_data.active_handles.len() + channel_data.active_streaming_handles.len();
        
        // Check cooldown
        if let Some(cooldown) = channel_data.cooldown_duration {
            if let Some(last_played) = channel_data.last_played {
                let elapsed = last_played.elapsed();
                if elapsed < cooldown {
                    let remaining = cooldown - elapsed;
                    tracing::trace!("Channel '{}' cooldown: ignoring '{}' ({}ms remaining)", 
                        channel, name, remaining.as_millis());
                    return Ok(());
                }
            }
        }
        
        // Apply channel policy
        match channel_data.policy {
            PlaybackPolicy::Exclusive => {
                tracing::debug!("Channel '{}' policy=Exclusive: stopping {} active sounds before playing '{}'", 
                    channel, active_count, name);
                // Stop all existing sounds in this channel
                for handle in &mut channel_data.active_handles {
                    handle.stop(Tween::default());
                }
                channel_data.active_handles.clear();
                for handle in &mut channel_data.active_streaming_handles {
                    handle.stop(Tween::default());
                }
                channel_data.active_streaming_handles.clear();
            }
            PlaybackPolicy::IgnoreIfPlaying => {
                // Don't play if any sound is still active in this channel
                if !channel_data.active_handles.is_empty() || !channel_data.active_streaming_handles.is_empty() {
                    tracing::trace!("Channel '{}' policy=IgnoreIfPlaying: ignoring '{}' (has {} active sounds)", 
                        channel, name, active_count);
                    return Ok(());
                }
                tracing::trace!("Channel '{}' policy=IgnoreIfPlaying: playing '{}' (no active sounds)", 
                    channel, name);
            }
            PlaybackPolicy::Overlap => {
                tracing::trace!("Channel '{}' policy=Overlap: playing '{}' (current active: {})", 
                    channel, name, active_count);
            }
        }

        // Try preloaded SFX first (fast path)
        if let Some(sound_data) = self.preloaded_sounds.get(name) {
            let handle = self.manager.play(sound_data.clone())?;
            channel_data.active_handles.push(handle);
            channel_data.last_played = Some(Instant::now());
            let new_total = channel_data.active_handles.len() + channel_data.active_streaming_handles.len();
            tracing::trace!("✓ Played sound '{}' in channel '{}' (active: {})", name, channel, new_total);
            return Ok(());
        }

        // Try on-demand music loading as static sound
        if let Some(path) = self.music_paths.get(name) {
            let sound_data = StaticSoundData::from_file(path)?;
            let handle = self.manager.play(sound_data)?;
            channel_data.active_handles.push(handle);
            channel_data.last_played = Some(Instant::now()); // Record timestamp
            let new_total = channel_data.active_handles.len() + channel_data.active_streaming_handles.len();
            tracing::trace!("✓ Played music '{}' on-demand in channel '{}' (active sounds: {})", 
                name, channel, new_total);
            return Ok(());
        }

        tracing::warn!("Audio file '{}' not found in SFX or music", name);
        Ok(())
    }

    /// Play background music in a specific channel (typically looped)
    pub fn play_background_music_in_channel(
        &mut self,
        channel: &str,
        name: &str,
        volume: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.cleanup_finished_sounds();

        // Get or create channel with default Exclusive policy for music
        if !self.channels.contains_key(channel) {
            tracing::trace!("Creating new music channel '{}' with default Exclusive policy", channel);
            self.set_channel_policy(channel, PlaybackPolicy::Exclusive);
        }

        let channel_data = self.channels.get_mut(channel).unwrap();
        let active_count = channel_data.active_handles.len() + channel_data.active_streaming_handles.len();

        // Apply channel policy
        match channel_data.policy {
            PlaybackPolicy::Exclusive => {
                tracing::debug!("Music channel '{}' policy=Exclusive: stopping {} active sounds before playing '{}'", 
                    channel, active_count, name);
                // Stop all existing sounds in this channel
                for handle in &mut channel_data.active_handles {
                    handle.stop(Tween::default());
                }
                channel_data.active_handles.clear();
                for handle in &mut channel_data.active_streaming_handles {
                    handle.stop(Tween::default());
                }
                channel_data.active_streaming_handles.clear();
            }
            PlaybackPolicy::IgnoreIfPlaying => {
                if !channel_data.active_handles.is_empty() || !channel_data.active_streaming_handles.is_empty() {
                    tracing::trace!("Music channel '{}' policy=IgnoreIfPlaying: ignoring '{}' (has {} active sounds)", 
                        channel, name, active_count);
                    return Ok(());
                }
                tracing::trace!("Music channel '{}' policy=IgnoreIfPlaying: playing '{}' (no active sounds)", 
                    channel, name);
            }
            PlaybackPolicy::Overlap => {
                tracing::trace!("Music channel '{}' policy=Overlap: playing '{}' (current active: {})", 
                    channel, name, active_count);
            }
        }

        // Try on-demand music loading
        if let Some(path) = self.music_paths.get(name) {
            tracing::trace!("Playing theme '{}' with volume: {:?} in channel '{}'", name, volume, channel);
            let start = std::time::Instant::now();
            let sound_data = StreamingSoundData::from_file(path)?
                .with_settings(StreamingSoundSettings::new().loop_region(..).volume(-10.0));

            let handle = self.manager.play(sound_data)?;
            channel_data.active_streaming_handles.push(handle);
            
            let duration = start.elapsed();
            let new_total = channel_data.active_handles.len() + channel_data.active_streaming_handles.len();
            tracing::trace!("Music loading took: {:?}", duration);
            tracing::trace!("✓ Played streaming music '{}' in channel '{}' (active sounds: {})", 
                name, channel, new_total);
            return Ok(());
        }

        tracing::warn!("Audio file '{}' not found in music", name);
        Ok(())
    }

    /// Legacy method - plays sound with default overlap behavior
    pub fn play_sound(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.play_sound_in_channel("default", name)
    }

    /// Legacy method - plays background music with exclusive behavior  
    pub fn play_background_music(&mut self, name: &str, volume: f32) -> Result<(), Box<dyn std::error::Error>> {
        self.play_background_music_in_channel("music", name, volume)
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

impl std::fmt::Debug for AudioManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioManager")
            .field("preloaded_sounds_count", &self.preloaded_sounds.len())
            .field("music_paths_count", &self.music_paths.len())
            .field("channels_count", &self.channels.len())
            .finish()
    }
}

impl EventHandler<AudioEvent> for AudioManager {
    fn handle(&mut self, event: &AudioEvent) {
        match event {
            AudioEvent::PlayerWalk => {
                // Set policy once - no cooldown needed since game logic now controls frequency
                if !self.channels.contains_key("movement") {
                    self.set_channel_policy("movement", PlaybackPolicy::Overlap);
                    tracing::trace!("🎵 Initialized movement channel with Overlap policy");
                }
                if let Err(e) = self.play_sound_in_channel("movement", "sfx_slime_bounce") {
                    tracing::warn!("Failed to play footstep sound: {}", e);
                }
            }
            AudioEvent::PlayerCollision => {
                // Set policy once - no cooldown needed since game logic now controls frequency
                if !self.channels.contains_key("collision") {
                    self.set_channel_policy("collision", PlaybackPolicy::Exclusive);
                    tracing::trace!("💥 Initialized collision channel with Exclusive policy");
                }
                if let Err(e) = self.play_sound_in_channel("collision", "sfx_hit2") {
                    tracing::warn!("Failed to play collision sound: {}", e);
                }
            }
            AudioEvent::BackgroundMusic(name) => {
                // Background music uses Exclusive by default (only one track at a time)
                if let Err(e) = self.play_background_music_in_channel("music", name, 0.3) {
                    tracing::warn!("Failed to play background music '{}': {}", name, e);
                }
            }
        }
    }
}
