use kira::{
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
    sound::streaming::{StreamingSoundData, StreamingSoundHandle, StreamingSoundSettings},
    sound::FromFileError,
    AudioManager as KiraAudioManager, AudioManagerSettings, Decibels, Tween,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use toki_core::{
    game::{AudioChannel as AudioEventChannel, AudioEvent},
    project_assets::{discover_audio_files, ProjectAssetError},
    EventHandler,
};

use crate::systems::asset_loading::common_preloaded_sfx_names;

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
struct ActiveStaticSound {
    handle: StaticSoundHandle,
    base_gain: f32,
}

#[derive(Debug)]
struct ActiveStreamingSound {
    handle: StreamingSoundHandle<FromFileError>,
    base_gain: f32,
}

#[derive(Debug)]
struct AudioChannel {
    policy: PlaybackPolicy,
    active_handles: Vec<ActiveStaticSound>,
    active_streaming_handles: Vec<ActiveStreamingSound>,
    last_played: Option<Instant>,
    cooldown_duration: Option<Duration>,
}

/// Asset discovery and caching for audio files.
pub(crate) struct AudioAssetCache {
    assets_root: PathBuf,
    preloaded_sounds: HashMap<String, StaticSoundData>,
    sfx_paths: HashMap<String, String>,
    music_paths: HashMap<String, String>,
}

impl AudioAssetCache {
    fn new(assets_root: PathBuf) -> Self {
        Self {
            assets_root,
            preloaded_sounds: HashMap::new(),
            sfx_paths: HashMap::new(),
            music_paths: HashMap::new(),
        }
    }

    fn scan_and_preload_sfx(
        &mut self,
        preload_names: &[String],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let sfx_dir = self.assets_root.join("assets").join("audio").join("sfx");

        if !Path::new(&sfx_dir).exists() {
            tracing::warn!("SFX directory not found: {}", sfx_dir.display());
            return Ok(());
        }

        let inventory = classify_sfx_inventory(&discover_supported_audio_assets(&sfx_dir)?, preload_names);
        for (name, path) in inventory.all_paths {
            let path_str = path.to_string_lossy().to_string();
            self.sfx_paths.insert(name.clone(), path_str.clone());
            if inventory.preloaded_names.contains(&name) {
                if let Err(e) = self.preload_sound(&name, &path_str) {
                    tracing::warn!(
                        "Failed to preload SFX '{}':
  {}",
                        name,
                        e
                    );
                }
            }
        }

        tracing::info!(
            "Preloaded {} hot SFX files ({} discovered)",
            self.preloaded_sounds.len(),
            self.sfx_paths.len()
        );
        Ok(())
    }

    fn scan_music_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let music_dir = self.assets_root.join("assets").join("audio").join("music");

        if !Path::new(&music_dir).exists() {
            tracing::warn!("Music directory not found: {}", music_dir.display());
            return Ok(());
        }

        for (name, path) in discover_supported_audio_assets(&music_dir)? {
            let path_str = path.to_string_lossy().to_string();
            self.music_paths.insert(name, path_str);
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

    fn get_preloaded_sound(&self, name: &str) -> Option<&StaticSoundData> {
        self.preloaded_sounds.get(name)
    }

    fn get_sfx_path(&self, name: &str) -> Option<&String> {
        self.sfx_paths.get(name)
    }

    fn get_music_path(&self, name: &str) -> Option<&String> {
        self.music_paths.get(name)
    }

    fn insert_preloaded_sound(&mut self, name: String, data: StaticSoundData) {
        self.preloaded_sounds.insert(name, data);
    }

    fn sfx_paths(&self) -> &HashMap<String, String> {
        &self.sfx_paths
    }

    fn music_paths(&self) -> &HashMap<String, String> {
        &self.music_paths
    }
}

/// Audio playback state: kira manager, channels, and volume settings.
pub(crate) struct AudioPlaybackState {
    manager: KiraAudioManager,
    channels: HashMap<String, AudioChannel>,
    master_volume_percent: u8,
    channel_volume_percents: HashMap<String, u8>,
    listener_position: Option<glam::IVec2>,
}

impl AudioPlaybackState {
    fn new(manager: KiraAudioManager) -> Self {
        Self {
            manager,
            channels: HashMap::new(),
            master_volume_percent: 100,
            channel_volume_percents: HashMap::new(),
            listener_position: None,
        }
    }

    fn set_channel_policy(&mut self, channel: &str, policy: PlaybackPolicy) {
        tracing::trace!("Setting channel '{}' policy to {:?}", channel, policy);
        self.channels.insert(
            channel.to_string(),
            AudioChannel {
                policy,
                active_handles: Vec::new(),
                active_streaming_handles: Vec::new(),
                last_played: None,
                cooldown_duration: None,
            },
        );
    }

    fn set_channel_volume_percent(&mut self, channel: &str, percent: u8) {
        let percent = percent.min(100);
        self.channel_volume_percents
            .insert(channel.to_string(), percent);
        let channel_gain = self.channel_volume_for(channel);
        if let Some(channel_data) = self.channels.get_mut(channel) {
            for active in &mut channel_data.active_handles {
                active.handle.set_volume(
                    amplitude_to_decibels(active.base_gain) + channel_gain,
                    Tween::default(),
                );
            }
            for active in &mut channel_data.active_streaming_handles {
                active.handle.set_volume(
                    amplitude_to_decibels(active.base_gain) + channel_gain,
                    Tween::default(),
                );
            }
        }
    }

    fn set_master_volume_percent(&mut self, percent: u8) {
        self.master_volume_percent = percent.min(100);
    }

    fn set_listener_position(&mut self, listener_position: Option<glam::IVec2>) {
        self.listener_position = listener_position;
    }

    fn listener_position(&self) -> Option<glam::IVec2> {
        self.listener_position
    }

    fn set_channel_cooldown(&mut self, channel: &str, cooldown: Duration) {
        if let Some(channel_data) = self.channels.get_mut(channel) {
            channel_data.cooldown_duration = Some(cooldown);
            tracing::trace!("Set cooldown for channel '{}' to {:?}", channel, cooldown);
        } else {
            self.channels.insert(
                channel.to_string(),
                AudioChannel {
                    policy: PlaybackPolicy::Overlap,
                    active_handles: Vec::new(),
                    active_streaming_handles: Vec::new(),
                    last_played: None,
                    cooldown_duration: Some(cooldown),
                },
            );
            tracing::trace!("Created channel '{}' with cooldown {:?}", channel, cooldown);
        }
    }

    fn stop_channel(&mut self, channel: &str) {
        if let Some(channel_data) = self.channels.get_mut(channel) {
            let total_stopped =
                channel_data.active_handles.len() + channel_data.active_streaming_handles.len();
            tracing::debug!("Stopping {} sounds in channel '{}'", total_stopped, channel);

            for handle in &mut channel_data.active_handles {
                handle.handle.stop(Tween::default());
            }
            channel_data.active_handles.clear();

            for handle in &mut channel_data.active_streaming_handles {
                handle.handle.stop(Tween::default());
            }
            channel_data.active_streaming_handles.clear();
        } else {
            tracing::trace!("Channel '{}' not found when trying to stop", channel);
        }
    }

    fn cleanup_finished_sounds(&mut self) {
        for (channel_name, channel) in &mut self.channels {
            let initial_static = channel.active_handles.len();
            let initial_streaming = channel.active_streaming_handles.len();

            if initial_static > 0 || initial_streaming > 0 {
                tracing::trace!(
                    "Channel '{}' before cleanup: {} static, {} streaming",
                    channel_name,
                    initial_static,
                    initial_streaming
                );
                for (i, handle) in channel.active_handles.iter().enumerate() {
                    tracing::trace!("  Static handle {}: state={:?}", i, handle.handle.state());
                }
                for (i, handle) in channel.active_streaming_handles.iter().enumerate() {
                    tracing::trace!("  Streaming handle {}: state={:?}", i, handle.handle.state());
                }
            }

            channel
                .active_handles
                .retain(|handle| handle.handle.state() != kira::sound::PlaybackState::Stopped);
            channel
                .active_streaming_handles
                .retain(|handle| handle.handle.state() != kira::sound::PlaybackState::Stopped);

            let removed_static = initial_static - channel.active_handles.len();
            let removed_streaming = initial_streaming - channel.active_streaming_handles.len();
            let total_removed = removed_static + removed_streaming;

            if total_removed > 0 {
                tracing::trace!(
                    "Cleaned up {} finished sounds from channel '{}' (static: {}, streaming: {})",
                    total_removed,
                    channel_name,
                    removed_static,
                    removed_streaming
                );
            }
        }
    }

    fn channel_volume_for(&self, channel: &str) -> Decibels {
        let master = percent_to_decibels(self.master_volume_percent);
        let percent = self
            .channel_volume_percents
            .get(channel)
            .copied()
            .unwrap_or(100);
        master + percent_to_decibels(percent)
    }

    fn has_channel(&self, channel: &str) -> bool {
        self.channels.contains_key(channel)
    }

    fn get_channel_mut(&mut self, channel: &str) -> Option<&mut AudioChannel> {
        self.channels.get_mut(channel)
    }

    fn play_static_sound(
        &mut self,
        sound_data: StaticSoundData,
    ) -> Result<StaticSoundHandle, Box<dyn std::error::Error>> {
        Ok(self.manager.play(sound_data)?)
    }

    fn play_streaming_sound(
        &mut self,
        sound_data: StreamingSoundData<FromFileError>,
    ) -> Result<StreamingSoundHandle<FromFileError>, Box<dyn std::error::Error>> {
        Ok(self.manager.play(sound_data)?)
    }
}

pub struct AudioManager {
    cache: AudioAssetCache,
    playback: AudioPlaybackState,
}

impl AudioManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let current_dir = std::env::current_dir()?;
        Self::new_with_assets_root(current_dir)
    }

    pub fn new_with_assets_root(
        assets_root: impl Into<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let preload_names = common_preloaded_sfx_names();
        Self::new_with_assets_root_and_preload_names(assets_root, &preload_names)
    }

    pub fn new_with_assets_root_and_preload_names(
        assets_root: impl Into<PathBuf>,
        preload_names: &[String],
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let manager = KiraAudioManager::new(AudioManagerSettings::default())?;
        let assets_root = assets_root.into();

        let mut cache = AudioAssetCache::new(assets_root);
        let playback = AudioPlaybackState::new(manager);

        cache.scan_and_preload_sfx(preload_names)?;
        cache.scan_music_files()?;

        Ok(Self { cache, playback })
    }

    /// Create or configure an audio channel with a specific playback policy
    pub fn set_channel_policy(&mut self, channel: &str, policy: PlaybackPolicy) {
        self.playback.set_channel_policy(channel, policy);
    }

    pub fn set_channel_volume_percent(&mut self, channel: &str, percent: u8) {
        self.playback.set_channel_volume_percent(channel, percent);
    }

    pub fn set_master_volume_percent(&mut self, percent: u8) {
        self.playback.set_master_volume_percent(percent);
    }

    pub fn set_listener_position(&mut self, listener_position: Option<glam::IVec2>) {
        self.playback.set_listener_position(listener_position);
    }

    /// Set a cooldown duration for a channel to prevent rapid-fire sounds
    pub fn set_channel_cooldown(&mut self, channel: &str, cooldown: Duration) {
        self.playback.set_channel_cooldown(channel, cooldown);
    }

    /// Stop all sounds in a specific channel
    pub fn stop_channel(&mut self, channel: &str) {
        self.playback.stop_channel(channel);
    }

    /// Play sound in a specific channel with channel policy enforcement
    pub fn play_sound_in_channel(
        &mut self,
        channel: &str,
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.play_sound_in_channel_with_gain(channel, name, 1.0)
    }

    pub fn play_sound_in_channel_with_gain(
        &mut self,
        channel: &str,
        name: &str,
        gain: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if gain <= 0.0 {
            return Ok(());
        }
        self.playback.cleanup_finished_sounds();

        // Get or create channel with default Overlap policy
        if !self.playback.has_channel(channel) {
            tracing::trace!(
                "Creating new channel '{}' with default Overlap policy",
                channel
            );
            self.playback
                .set_channel_policy(channel, PlaybackPolicy::Overlap);
        }

        let channel_gain = self.playback.channel_volume_for(channel) + amplitude_to_decibels(gain);
        let channel_data = self.playback.get_channel_mut(channel).unwrap();
        let active_count =
            channel_data.active_handles.len() + channel_data.active_streaming_handles.len();

        // Check cooldown
        if let Some(cooldown) = channel_data.cooldown_duration {
            if let Some(last_played) = channel_data.last_played {
                let elapsed = last_played.elapsed();
                if elapsed < cooldown {
                    let remaining = cooldown - elapsed;
                    tracing::trace!(
                        "Channel '{}' cooldown: ignoring '{}' ({}ms remaining)",
                        channel,
                        name,
                        remaining.as_millis()
                    );
                    return Ok(());
                }
            }
        }

        // Apply channel policy
        match channel_data.policy {
            PlaybackPolicy::Exclusive => {
                tracing::debug!(
                    "Channel '{}' policy=Exclusive: stopping {} active sounds before playing '{}'",
                    channel,
                    active_count,
                    name
                );
                for handle in &mut channel_data.active_handles {
                    handle.handle.stop(Tween::default());
                }
                channel_data.active_handles.clear();
                for handle in &mut channel_data.active_streaming_handles {
                    handle.handle.stop(Tween::default());
                }
                channel_data.active_streaming_handles.clear();
            }
            PlaybackPolicy::IgnoreIfPlaying => {
                if !channel_data.active_handles.is_empty()
                    || !channel_data.active_streaming_handles.is_empty()
                {
                    tracing::trace!(
                        "Channel '{}' policy=IgnoreIfPlaying: ignoring '{}' (has {} active sounds)",
                        channel,
                        name,
                        active_count
                    );
                    return Ok(());
                }
                tracing::trace!(
                    "Channel '{}' policy=IgnoreIfPlaying: playing '{}' (no active sounds)",
                    channel,
                    name
                );
            }
            PlaybackPolicy::Overlap => {
                tracing::trace!(
                    "Channel '{}' policy=Overlap: playing '{}' (current active: {})",
                    channel,
                    name,
                    active_count
                );
            }
        }

        // Try preloaded SFX first (fast path)
        if let Some(sound_data) = self.cache.get_preloaded_sound(name) {
            let handle = self
                .playback
                .play_static_sound(sound_data.volume(channel_gain))?;
            let channel_data = self.playback.get_channel_mut(channel).unwrap();
            channel_data.active_handles.push(ActiveStaticSound {
                handle,
                base_gain: gain,
            });
            channel_data.last_played = Some(Instant::now());
            let new_total =
                channel_data.active_handles.len() + channel_data.active_streaming_handles.len();
            tracing::trace!(
                "✓ Played sound '{}' in channel '{}' (active: {})",
                name,
                channel,
                new_total
            );
            return Ok(());
        }

        // Try loading from SFX paths (lazy load and cache)
        if let Some(path) = self.cache.get_sfx_path(name).cloned() {
            let sound_data = StaticSoundData::from_file(&path)?;
            self.cache
                .insert_preloaded_sound(name.to_string(), sound_data.clone());
            let handle = self
                .playback
                .play_static_sound(sound_data.volume(channel_gain))?;
            let channel_data = self.playback.get_channel_mut(channel).unwrap();
            channel_data.active_handles.push(ActiveStaticSound {
                handle,
                base_gain: gain,
            });
            channel_data.last_played = Some(Instant::now());
            let new_total =
                channel_data.active_handles.len() + channel_data.active_streaming_handles.len();
            tracing::trace!(
                "✓ Loaded and cached SFX '{}' on-demand in channel '{}' (active sounds: {})",
                name,
                channel,
                new_total
            );
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
        self.playback.cleanup_finished_sounds();

        // Get or create channel with default Exclusive policy for music
        if !self.playback.has_channel(channel) {
            tracing::trace!(
                "Creating new music channel '{}' with default Exclusive policy",
                channel
            );
            self.playback
                .set_channel_policy(channel, PlaybackPolicy::Exclusive);
        }

        let channel_gain = self.playback.channel_volume_for(channel);
        let channel_data = self.playback.get_channel_mut(channel).unwrap();
        let active_count =
            channel_data.active_handles.len() + channel_data.active_streaming_handles.len();

        // Apply channel policy
        match channel_data.policy {
            PlaybackPolicy::Exclusive => {
                tracing::debug!("Music channel '{}' policy=Exclusive: stopping {} active sounds before playing '{}'",
                    channel, active_count, name);
                for handle in &mut channel_data.active_handles {
                    handle.handle.stop(Tween::default());
                }
                channel_data.active_handles.clear();
                for handle in &mut channel_data.active_streaming_handles {
                    handle.handle.stop(Tween::default());
                }
                channel_data.active_streaming_handles.clear();
            }
            PlaybackPolicy::IgnoreIfPlaying => {
                if !channel_data.active_handles.is_empty()
                    || !channel_data.active_streaming_handles.is_empty()
                {
                    tracing::trace!("Music channel '{}' policy=IgnoreIfPlaying: ignoring '{}' (has {} active sounds)",
                        channel, name, active_count);
                    return Ok(());
                }
                tracing::trace!(
                    "Music channel '{}' policy=IgnoreIfPlaying: playing '{}' (no active sounds)",
                    channel,
                    name
                );
            }
            PlaybackPolicy::Overlap => {
                tracing::trace!(
                    "Music channel '{}' policy=Overlap: playing '{}' (current active: {})",
                    channel,
                    name,
                    active_count
                );
            }
        }

        // Try on-demand music loading
        if let Some(path) = self.cache.get_music_path(name).cloned() {
            tracing::trace!(
                "Playing theme '{}' with volume: {:?} in channel '{}'",
                name,
                volume,
                channel
            );
            let start = std::time::Instant::now();
            let sound_data = StreamingSoundData::from_file(&path)?.with_settings(
                StreamingSoundSettings::new()
                    .loop_region(..)
                    .volume(amplitude_to_decibels(volume) + channel_gain),
            );

            let handle = self.playback.play_streaming_sound(sound_data)?;
            let channel_data = self.playback.get_channel_mut(channel).unwrap();
            channel_data.active_streaming_handles.push(ActiveStreamingSound {
                handle,
                base_gain: volume,
            });

            let duration = start.elapsed();
            let new_total =
                channel_data.active_handles.len() + channel_data.active_streaming_handles.len();
            tracing::trace!("Music loading took: {:?}", duration);
            tracing::trace!(
                "✓ Played streaming music '{}' in channel '{}' (active sounds: {})",
                name,
                channel,
                new_total
            );
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
    pub fn play_background_music(
        &mut self,
        name: &str,
        volume: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.play_background_music_in_channel("music", name, volume)
    }

    // Debug helper to list available sounds
    pub fn list_available_sounds(&self) {
        tracing::info!(
            "Available SFX: {:?}",
            self.cache.sfx_paths().keys().collect::<Vec<_>>()
        );
        tracing::info!(
            "Available Music: {:?}",
            self.cache.music_paths().keys().collect::<Vec<_>>()
        );
    }
}

fn spatial_attenuation(
    listener_position: Option<glam::IVec2>,
    source_position: Option<glam::IVec2>,
    hearing_radius: Option<u32>,
) -> Option<f32> {
    let (Some(listener), Some(source), Some(radius)) =
        (listener_position, source_position, hearing_radius)
    else {
        return Some(1.0);
    };

    if radius == 0 {
        return None;
    }

    let distance = (source - listener).as_vec2().length();
    let normalized = (distance / radius as f32).clamp(0.0, 1.0);
    if normalized >= 1.0 {
        return None;
    }

    let smoothstep = normalized * normalized * (3.0 - 2.0 * normalized);
    Some(1.0 - smoothstep)
}

fn discover_supported_audio_assets(dir: &Path) -> Result<Vec<(String, PathBuf)>, std::io::Error> {
    let discovered = discover_audio_files(dir).map_err(|error| match error {
        ProjectAssetError::Io(io) => io,
        other => std::io::Error::other(other.to_string()),
    })?;

    Ok(discovered
        .into_iter()
        .map(|asset| (asset.name, asset.path))
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SfxInventory {
    all_paths: Vec<(String, PathBuf)>,
    preloaded_names: Vec<String>,
}

fn classify_sfx_inventory(
    discovered: &[(String, PathBuf)],
    preload_names: &[String],
) -> SfxInventory {
    let preload_name_set = preload_names
        .iter()
        .collect::<std::collections::HashSet<_>>();
    let preloaded_names = discovered
        .iter()
        .filter_map(|(name, _)| preload_name_set.contains(name).then_some(name.clone()))
        .collect::<Vec<_>>();
    SfxInventory {
        all_paths: discovered.to_vec(),
        preloaded_names,
    }
}

fn percent_to_decibels(percent: u8) -> Decibels {
    if percent == 0 {
        return Decibels::SILENCE;
    }
    amplitude_to_decibels(percent as f32 / 100.0)
}

fn amplitude_to_decibels(amplitude: f32) -> Decibels {
    if amplitude <= 0.0 {
        return Decibels::SILENCE;
    }
    Decibels((20.0 * amplitude.log10()).max(Decibels::SILENCE.0))
}

impl std::fmt::Debug for AudioManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioManager")
            .field("preloaded_sounds_count", &self.cache.preloaded_sounds.len())
            .field("sfx_paths_count", &self.cache.sfx_paths.len())
            .field("music_paths_count", &self.cache.music_paths.len())
            .field("channels_count", &self.playback.channels.len())
            .field(
                "master_volume_percent",
                &self.playback.master_volume_percent,
            )
            .field(
                "channel_volume_percents",
                &self.playback.channel_volume_percents,
            )
            .finish()
    }
}

impl EventHandler<AudioEvent> for AudioManager {
    fn handle(&mut self, event: &AudioEvent) {
        match event {
            AudioEvent::PlaySound {
                channel,
                sound_id,
                source_position,
                hearing_radius,
            } => {
                let (channel_name, policy) = match channel {
                    AudioEventChannel::Movement => ("movement", PlaybackPolicy::Overlap),
                    AudioEventChannel::Collision => ("collision", PlaybackPolicy::Exclusive),
                };

                if !self.playback.has_channel(channel_name) {
                    self.playback.set_channel_policy(channel_name, policy);
                    tracing::trace!(
                        "Initialized '{}' channel with {:?} policy",
                        channel_name,
                        policy
                    );
                }

                let Some(gain) = spatial_attenuation(
                    self.playback.listener_position(),
                    *source_position,
                    *hearing_radius,
                ) else {
                    return;
                };

                if let Err(e) = self.play_sound_in_channel_with_gain(channel_name, sound_id, gain) {
                    tracing::warn!(
                        "Failed to play sound '{}' in '{}' channel: {}",
                        sound_id,
                        channel_name,
                        e
                    );
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

#[cfg(test)]
#[path = "audio_manager_tests.rs"]
mod tests;
