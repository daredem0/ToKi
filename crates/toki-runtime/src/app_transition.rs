use crate::app::RuntimeTransitionOptions;
use crate::systems::AudioManager;
use toki_core::SceneSwitchRequest;

const BASE_MUSIC_GAIN: f32 = 0.3;
const MUSIC_CHANNEL_A: &str = "music_a";
const MUSIC_CHANNEL_B: &str = "music_b";

pub(crate) trait TransitionAudioSink {
    fn play_background_music_in_channel(
        &mut self,
        channel: &str,
        track_id: &str,
        volume: f32,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn set_channel_volume_percent(&mut self, channel: &str, percent: u8);
    fn stop_channel(&mut self, channel: &str);
}

impl TransitionAudioSink for AudioManager {
    fn play_background_music_in_channel(
        &mut self,
        channel: &str,
        track_id: &str,
        volume: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.play_background_music_in_channel(channel, track_id, volume)
    }

    fn set_channel_volume_percent(&mut self, channel: &str, percent: u8) {
        self.set_channel_volume_percent(channel, percent);
    }

    fn stop_channel(&mut self, channel: &str) {
        self.stop_channel(channel);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MusicChannelSlot {
    A,
    B,
}

impl MusicChannelSlot {
    fn channel_name(self) -> &'static str {
        match self {
            Self::A => MUSIC_CHANNEL_A,
            Self::B => MUSIC_CHANNEL_B,
        }
    }

    fn alternate(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MusicPlayback {
    track_id: String,
    slot: MusicChannelSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TransitionPhase {
    Idle,
    FadingOut {
        request: SceneSwitchRequest,
        target_track_id: Option<String>,
        same_track: bool,
        elapsed_ms: u32,
        outgoing_music: Option<MusicPlayback>,
    },
    FadingIn {
        elapsed_ms: u32,
        fade_in_music: Option<MusicPlayback>,
        preserve_same_track: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TransitionAdvance {
    None,
    ReadyToSwap(SceneSwitchRequest),
    Completed,
}

#[derive(Debug, Clone)]
pub(crate) struct SceneTransitionController {
    fade_duration_ms: u32,
    phase: TransitionPhase,
    current_music: Option<MusicPlayback>,
}

impl SceneTransitionController {
    pub(crate) fn new(options: RuntimeTransitionOptions) -> Self {
        Self {
            fade_duration_ms: options.fade_duration_ms.max(1),
            phase: TransitionPhase::Idle,
            current_music: None,
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        !matches!(self.phase, TransitionPhase::Idle)
    }

    pub(crate) fn fade_alpha(&self) -> f32 {
        match &self.phase {
            TransitionPhase::Idle => 0.0,
            TransitionPhase::FadingOut { elapsed_ms, .. } => {
                (*elapsed_ms as f32 / self.fade_duration_ms as f32).clamp(0.0, 1.0)
            }
            TransitionPhase::FadingIn { elapsed_ms, .. } => {
                1.0 - (*elapsed_ms as f32 / self.fade_duration_ms as f32).clamp(0.0, 1.0)
            }
        }
    }

    pub(crate) fn request_scene_switch(
        &mut self,
        request: SceneSwitchRequest,
        target_track_id: Option<String>,
    ) -> bool {
        if self.is_active() {
            return false;
        }

        let outgoing_music = self.current_music.clone();
        let same_track = outgoing_music
            .as_ref()
            .map(|music| Some(music.track_id.as_str()) == target_track_id.as_deref())
            .unwrap_or(false);

        self.phase = TransitionPhase::FadingOut {
            request,
            target_track_id,
            same_track,
            elapsed_ms: 0,
            outgoing_music,
        };
        true
    }

    pub(crate) fn prime_scene_music(
        &mut self,
        audio: &mut impl TransitionAudioSink,
        track_id: Option<&str>,
        base_music_percent: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        audio.set_channel_volume_percent(MUSIC_CHANNEL_A, 0);
        audio.set_channel_volume_percent(MUSIC_CHANNEL_B, 0);

        let Some(track_id) = track_id else {
            self.current_music = None;
            return Ok(());
        };

        audio.play_background_music_in_channel(MUSIC_CHANNEL_A, track_id, BASE_MUSIC_GAIN)?;
        audio.set_channel_volume_percent(MUSIC_CHANNEL_A, base_music_percent);
        self.current_music = Some(MusicPlayback {
            track_id: track_id.to_string(),
            slot: MusicChannelSlot::A,
        });
        Ok(())
    }

    pub(crate) fn ensure_scene_music(
        &mut self,
        audio: &mut impl TransitionAudioSink,
        track_id: Option<&str>,
        base_music_percent: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_active() {
            return Ok(());
        }

        match (self.current_music.as_ref(), track_id) {
            (Some(current), Some(track_id)) if current.track_id == track_id => {
                audio.set_channel_volume_percent(current.slot.channel_name(), base_music_percent);
                Ok(())
            }
            (None, Some(track_id)) => self.prime_scene_music(audio, Some(track_id), base_music_percent),
            (Some(current), None) => {
                audio.stop_channel(current.slot.channel_name());
                self.current_music = None;
                Ok(())
            }
            (Some(current), Some(track_id)) => {
                audio.stop_channel(current.slot.channel_name());
                self.current_music = None;
                self.prime_scene_music(audio, Some(track_id), base_music_percent)
            }
            (None, None) => Ok(()),
        }
    }

    pub(crate) fn advance(
        &mut self,
        delta_ms: u32,
        audio: &mut impl TransitionAudioSink,
        base_music_percent: u8,
    ) -> TransitionAdvance {
        match &mut self.phase {
            TransitionPhase::Idle => TransitionAdvance::None,
            TransitionPhase::FadingOut {
                request,
                same_track,
                elapsed_ms,
                outgoing_music,
                ..
            } => {
                *elapsed_ms = elapsed_ms.saturating_add(delta_ms);
                let progress = (*elapsed_ms as f32 / self.fade_duration_ms as f32).clamp(0.0, 1.0);
                if !*same_track {
                    if let Some(outgoing_music) = outgoing_music.as_ref() {
                        audio.set_channel_volume_percent(
                            outgoing_music.slot.channel_name(),
                            scaled_percent(base_music_percent, 1.0 - progress),
                        );
                    }
                }
                if *elapsed_ms >= self.fade_duration_ms {
                    TransitionAdvance::ReadyToSwap(request.clone())
                } else {
                    TransitionAdvance::None
                }
            }
            TransitionPhase::FadingIn {
                elapsed_ms,
                fade_in_music,
                preserve_same_track,
            } => {
                *elapsed_ms = elapsed_ms.saturating_add(delta_ms);
                let progress = (*elapsed_ms as f32 / self.fade_duration_ms as f32).clamp(0.0, 1.0);
                if !*preserve_same_track {
                    if let Some(fade_in_music) = fade_in_music.as_ref() {
                        audio.set_channel_volume_percent(
                            fade_in_music.slot.channel_name(),
                            scaled_percent(base_music_percent, progress),
                        );
                    }
                }
                if *elapsed_ms >= self.fade_duration_ms {
                    self.phase = TransitionPhase::Idle;
                    TransitionAdvance::Completed
                } else {
                    TransitionAdvance::None
                }
            }
        }
    }

    pub(crate) fn complete_scene_switch(
        &mut self,
        audio: &mut impl TransitionAudioSink,
        success: bool,
        resolved_track_id: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let TransitionPhase::FadingOut {
            target_track_id,
            same_track,
            outgoing_music,
            ..
        } = &self.phase
        else {
            return Ok(());
        };

        let outgoing_music = outgoing_music.clone();
        let preserve_same_track = *same_track;
        let fallback_track_id = outgoing_music.as_ref().map(|music| music.track_id.as_str());
        let resolved_track_id = if success {
            resolved_track_id.or(target_track_id.as_deref())
        } else {
            fallback_track_id
        };

        let fade_in_music = if preserve_same_track {
            outgoing_music.clone()
        } else if let Some(track_id) = resolved_track_id {
            let next_slot = outgoing_music
                .as_ref()
                .map(|music| music.slot.alternate())
                .unwrap_or(MusicChannelSlot::A);
            audio.play_background_music_in_channel(next_slot.channel_name(), track_id, BASE_MUSIC_GAIN)?;
            audio.set_channel_volume_percent(next_slot.channel_name(), 0);
            if success {
                if let Some(outgoing_music) = outgoing_music.as_ref() {
                    audio.stop_channel(outgoing_music.slot.channel_name());
                }
            }
            Some(MusicPlayback {
                track_id: track_id.to_string(),
                slot: next_slot,
            })
        } else {
            if let Some(outgoing_music) = outgoing_music.as_ref() {
                audio.stop_channel(outgoing_music.slot.channel_name());
            }
            None
        };

        self.current_music = if success {
            fade_in_music.clone()
        } else {
            outgoing_music.clone()
        };
        self.phase = TransitionPhase::FadingIn {
            elapsed_ms: 0,
            fade_in_music,
            preserve_same_track,
        };
        Ok(())
    }
}

fn scaled_percent(base_percent: u8, factor: f32) -> u8 {
    ((base_percent as f32 * factor.clamp(0.0, 1.0)).round() as i32).clamp(0, 100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeAudioSink {
        plays: Vec<(String, String, f32)>,
        volume_changes: Vec<(String, u8)>,
        stops: Vec<String>,
    }

    impl TransitionAudioSink for FakeAudioSink {
        fn play_background_music_in_channel(
            &mut self,
            channel: &str,
            track_id: &str,
            volume: f32,
        ) -> Result<(), Box<dyn std::error::Error>> {
            self.plays
                .push((channel.to_string(), track_id.to_string(), volume));
            Ok(())
        }

        fn set_channel_volume_percent(&mut self, channel: &str, percent: u8) {
            self.volume_changes.push((channel.to_string(), percent));
        }

        fn stop_channel(&mut self, channel: &str) {
            self.stops.push(channel.to_string());
        }
    }

    #[test]
    fn transition_waits_for_fade_out_before_swap() {
        let mut controller = SceneTransitionController::new(RuntimeTransitionOptions {
            fade_duration_ms: 100,
        });
        let mut audio = FakeAudioSink::default();
        controller.request_scene_switch(
            SceneSwitchRequest {
                scene_name: "Scene B".to_string(),
                spawn_point_id: "entry_b".to_string(),
            },
            None,
        );

        assert!(matches!(
            controller.advance(50, &mut audio, 100),
            TransitionAdvance::None
        ));
        assert!(controller.fade_alpha() > 0.0);

        assert!(matches!(
            controller.advance(50, &mut audio, 100),
            TransitionAdvance::ReadyToSwap(SceneSwitchRequest { .. })
        ));
        assert_eq!(controller.fade_alpha(), 1.0);
    }

    #[test]
    fn same_track_transition_continues_without_restarting_music() {
        let mut controller = SceneTransitionController::new(RuntimeTransitionOptions {
            fade_duration_ms: 100,
        });
        let mut audio = FakeAudioSink::default();
        controller
            .prime_scene_music(&mut audio, Some("track_a"), 80)
            .expect("prime should succeed");
        audio.plays.clear();
        audio.stops.clear();
        audio.volume_changes.clear();

        controller.request_scene_switch(
            SceneSwitchRequest {
                scene_name: "Scene B".to_string(),
                spawn_point_id: "entry_b".to_string(),
            },
            Some("track_a".to_string()),
        );
        let _ = controller.advance(100, &mut audio, 80);
        controller
            .complete_scene_switch(&mut audio, true, Some("track_a"))
            .expect("complete should succeed");
        let _ = controller.advance(100, &mut audio, 80);

        assert!(audio.plays.is_empty());
        assert!(audio.stops.is_empty());
    }

    #[test]
    fn different_track_transition_starts_new_music_on_alternate_channel() {
        let mut controller = SceneTransitionController::new(RuntimeTransitionOptions {
            fade_duration_ms: 100,
        });
        let mut audio = FakeAudioSink::default();
        controller
            .prime_scene_music(&mut audio, Some("track_a"), 80)
            .expect("prime should succeed");
        audio.plays.clear();
        audio.stops.clear();
        audio.volume_changes.clear();

        controller.request_scene_switch(
            SceneSwitchRequest {
                scene_name: "Scene B".to_string(),
                spawn_point_id: "entry_b".to_string(),
            },
            Some("track_b".to_string()),
        );
        let _ = controller.advance(100, &mut audio, 80);
        controller
            .complete_scene_switch(&mut audio, true, Some("track_b"))
            .expect("complete should succeed");

        assert_eq!(
            audio.plays,
            vec![(MUSIC_CHANNEL_B.to_string(), "track_b".to_string(), BASE_MUSIC_GAIN)]
        );
        assert_eq!(audio.stops, vec![MUSIC_CHANNEL_A.to_string()]);
    }

    #[test]
    fn ensure_scene_music_starts_track_when_none_is_active() {
        let mut controller = SceneTransitionController::new(RuntimeTransitionOptions::default());
        let mut audio = FakeAudioSink::default();

        controller
            .ensure_scene_music(&mut audio, Some("track_a"), 75)
            .expect("ensure should succeed");

        assert_eq!(
            audio.plays,
            vec![(MUSIC_CHANNEL_A.to_string(), "track_a".to_string(), BASE_MUSIC_GAIN)]
        );
        assert!(audio
            .volume_changes
            .contains(&(MUSIC_CHANNEL_A.to_string(), 75)));
    }

    #[test]
    fn ensure_scene_music_does_not_restart_same_track() {
        let mut controller = SceneTransitionController::new(RuntimeTransitionOptions::default());
        let mut audio = FakeAudioSink::default();
        controller
            .prime_scene_music(&mut audio, Some("track_a"), 80)
            .expect("prime should succeed");
        audio.plays.clear();
        audio.stops.clear();
        audio.volume_changes.clear();

        controller
            .ensure_scene_music(&mut audio, Some("track_a"), 80)
            .expect("ensure should succeed");

        assert!(audio.plays.is_empty());
        assert!(audio.stops.is_empty());
        assert_eq!(
            audio.volume_changes,
            vec![(MUSIC_CHANNEL_A.to_string(), 80)]
        );
    }
}
