/// Data model for a single animation frame.
#[derive(Debug, Clone)]
pub struct Frame {
    pub index: u32,
    pub duration_ms: u32,
}

/// An animation consists of a sequence of frames, and a flag if it should loop.
#[derive(Debug, Clone)]
pub struct Animation {
    pub name: String,
    pub frames: Vec<Frame>,
    pub looped: bool,
}

/// Metadata about a sprite sheet (grid layout of frames).
#[derive(Debug, Clone)]
pub struct SpriteSheetMeta {
    pub frame_size: (u32, u32),
    pub sheet_size: (u32, u32),
    pub frame_count: u32,
}

/// Tracks and updates animation playback over time.
#[derive(Debug, Clone)]
pub struct Animator {
    pub current_frame: usize,
    pub elapsed_ms: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct SpriteFrame {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

impl Animator {
    pub fn new() -> Self {
        Self {
            current_frame: 0,
            elapsed_ms: 0,
        }
    }

    pub fn update(&mut self, delta_ms: u32, animation: &Animation) {
        if animation.frames.is_empty() {
            return;
        }

        self.elapsed_ms += delta_ms;
        let current = &animation.frames[self.current_frame];

        if self.elapsed_ms >= current.duration_ms {
            self.elapsed_ms -= current.duration_ms;
            self.current_frame += 1;

            if self.current_frame >= animation.frames.len() {
                self.current_frame = if animation.looped {
                    0
                } else {
                    animation.frames.len() - 1
                };
            }
        }
    }

    pub fn frame_index(&self, animation: &Animation) -> u32 {
        if animation.frames.is_empty() {
            0
        } else {
            animation.frames[self.current_frame].index
        }
    }

    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.elapsed_ms = 0;
    }
}

impl SpriteSheetMeta {
    pub fn uv_rect(&self, frame_index: u32) -> SpriteFrame {
        let cols = self.sheet_size.0 / self.frame_size.0;
        let fx = frame_index % cols;
        let fy = frame_index / cols;

        let fw = self.frame_size.0 as f32;
        let fh = self.frame_size.1 as f32;
        let sw = self.sheet_size.0 as f32;
        let sh = self.sheet_size.1 as f32;

        SpriteFrame {
            u0: fx as f32 * fw / sw,
            v0: fy as f32 * fh / sh,
            u1: (fx + 1) as f32 * fw / sw,
            v1: (fy + 1) as f32 * fh / sh,
        }
    }
}
