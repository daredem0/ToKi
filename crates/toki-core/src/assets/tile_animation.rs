use crate::animation::ClipPlayback;
use crate::assets::atlas::AtlasMeta;
use std::collections::HashMap;

/// Global clock that tracks animation playback for all animated tile definitions.
/// All instances of the same animated tile are synchronized.
#[derive(Debug, Clone)]
pub struct TileAnimationClock {
    playbacks: HashMap<String, ClipPlayback>,
    any_frame_changed: bool,
}

impl TileAnimationClock {
    pub fn new() -> Self {
        Self {
            playbacks: HashMap::new(),
            any_frame_changed: false,
        }
    }

    /// Sync playback entries with the atlas's animated tile definitions.
    pub fn sync_definitions(&mut self, atlas: &AtlasMeta) {
        // Remove stale entries
        self.playbacks
            .retain(|name, _| atlas.animated_tiles.contains_key(name));
        // Add missing entries (auto-start playback)
        for name in atlas.animated_tiles.keys() {
            self.playbacks.entry(name.clone()).or_insert_with(|| {
                let mut p = ClipPlayback::new();
                p.play();
                p
            });
        }
    }

    pub fn sync_definitions_from_iter<'a>(
        &mut self,
        atlases: impl IntoIterator<Item = &'a AtlasMeta>,
    ) {
        let atlases = atlases.into_iter().collect::<Vec<_>>();
        self.playbacks.retain(|name, _| {
            atlases
                .iter()
                .any(|atlas| atlas.animated_tiles.contains_key(name))
        });
        for atlas in atlases {
            for name in atlas.animated_tiles.keys() {
                self.playbacks.entry(name.clone()).or_insert_with(|| {
                    let mut p = ClipPlayback::new();
                    p.play();
                    p
                });
            }
        }
    }

    /// Advance all playbacks. Returns `true` if any frame changed.
    pub fn update(&mut self, delta_ms: f32, atlas: &AtlasMeta) -> bool {
        self.any_frame_changed = false;
        for (name, playback) in &mut self.playbacks {
            let Some(def) = atlas.animated_tiles.get(name) else {
                continue;
            };
            let loop_mode = def.loop_mode.into();
            let event = playback.update(
                delta_ms,
                def.frame_count(),
                |i| def.frame_duration_at(i),
                &loop_mode,
            );
            if matches!(
                event,
                crate::animation::PlaybackEvent::FrameChanged { .. }
                    | crate::animation::PlaybackEvent::LoopCompleted
            ) {
                self.any_frame_changed = true;
            }
        }
        self.any_frame_changed
    }

    pub fn update_from_iter<'a>(
        &mut self,
        delta_ms: f32,
        atlases: impl IntoIterator<Item = &'a AtlasMeta>,
    ) -> bool {
        let atlas_map = atlases
            .into_iter()
            .flat_map(|atlas| {
                atlas
                    .animated_tiles
                    .keys()
                    .map(move |name| (name.clone(), atlas))
            })
            .collect::<HashMap<_, _>>();
        self.any_frame_changed = false;
        for (name, playback) in &mut self.playbacks {
            let Some(atlas) = atlas_map.get(name) else {
                continue;
            };
            let Some(def) = atlas.animated_tiles.get(name) else {
                continue;
            };
            let loop_mode = def.loop_mode.into();
            let event = playback.update(
                delta_ms,
                def.frame_count(),
                |i| def.frame_duration_at(i),
                &loop_mode,
            );
            if matches!(
                event,
                crate::animation::PlaybackEvent::FrameChanged { .. }
                    | crate::animation::PlaybackEvent::LoopCompleted
            ) {
                self.any_frame_changed = true;
            }
        }
        self.any_frame_changed
    }

    /// Returns the resolved tile name for the current animation frame.
    pub fn current_frame_tile<'a>(&self, name: &str, atlas: &'a AtlasMeta) -> Option<&'a str> {
        let playback = self.playbacks.get(name)?;
        let def = atlas.animated_tiles.get(name)?;
        def.frames.get(playback.current_frame).map(String::as_str)
    }
}

impl Default for TileAnimationClock {
    fn default() -> Self {
        Self::new()
    }
}
