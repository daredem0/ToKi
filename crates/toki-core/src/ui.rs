use crate::text::{TextAnchor, TextItem, TextStyle};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl UiRect {
    pub fn center_x(&self) -> f32 {
        self.x + self.width * 0.5
    }

    pub fn center_y(&self) -> f32 {
        self.y + self.height * 0.5
    }

    pub fn inset(&self, amount: f32) -> Self {
        let inset = amount.max(0.0);
        let double = inset * 2.0;
        Self {
            x: self.x + inset,
            y: self.y + inset,
            width: (self.width - double).max(0.0),
            height: (self.height - double).max(0.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiTextBlock {
    pub content: String,
    pub position: glam::Vec2,
    pub anchor: TextAnchor,
    pub style: TextStyle,
    pub layer: i32,
}

impl UiTextBlock {
    pub fn to_text_item(&self) -> TextItem {
        TextItem::new_screen(self.content.clone(), self.position, self.style.clone())
            .with_anchor(self.anchor)
            .with_layer(self.layer)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiBlock {
    pub rect: UiRect,
    pub fill_color: Option<[f32; 4]>,
    pub border_color: Option<[f32; 4]>,
    pub border_thickness: f32,
    pub text: Option<UiTextBlock>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiComposition {
    pub blocks: Vec<UiBlock>,
}

impl UiComposition {
    pub fn push(&mut self, block: UiBlock) {
        self.blocks.push(block);
    }
}

pub fn transform_logical_ui_rect(rect: UiRect, origin: glam::Vec2, scale: f32) -> UiRect {
    UiRect {
        x: origin.x + rect.x * scale,
        y: origin.y + rect.y * scale,
        width: rect.width * scale,
        height: rect.height * scale,
    }
}

pub fn runtime_ui_text_scale(
    logical_viewport_size: glam::Vec2,
    presented_viewport_size: glam::Vec2,
) -> f32 {
    let reference_size = glam::Vec2::new(logical_viewport_size.x * 7.0, logical_viewport_size.y * 7.0);
    let size_ratio = (presented_viewport_size.x / reference_size.x)
        .min(presented_viewport_size.y / reference_size.y)
        .clamp(0.0, 1.0);

    0.06 + 0.94 * size_ratio.powf(1.15)
}

pub fn transform_logical_ui_composition(
    composition: &UiComposition,
    origin: glam::Vec2,
    scale: f32,
) -> UiComposition {
    transform_logical_ui_composition_with_scales(composition, origin, scale, scale)
}

pub fn transform_logical_ui_composition_with_scales(
    composition: &UiComposition,
    origin: glam::Vec2,
    geometry_scale: f32,
    text_scale: f32,
) -> UiComposition {
    let mut transformed = composition.clone();
    for block in &mut transformed.blocks {
        block.rect = transform_logical_ui_rect(block.rect, origin, geometry_scale);
        block.border_thickness *= geometry_scale;
        if let Some(text) = block.text.as_mut() {
            text.position = glam::Vec2::new(
                origin.x + text.position.x * geometry_scale,
                origin.y + text.position.y * geometry_scale,
            );
            text.style.size_px *= text_scale;
        }
    }
    transformed
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiAction {
    #[serde(alias = "close_menu")]
    CloseUi,
    #[serde(alias = "close_dialog")]
    CloseSurface,
    #[serde(alias = "open_screen", alias = "open_dialog")]
    OpenSurface {
        #[serde(alias = "screen_id", alias = "dialog_id")]
        surface_id: String,
    },
    Back,
    #[serde(alias = "exit_game")]
    ExitRuntime,
    OpenAudioSettings,
    OpenDisplaySettings,
    OpenGraphicsSettings,
    SaveGame {
        slot: u8,
    },
    LoadGame {
        slot: u8,
    },
    EmitEvent {
        event_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiCommand {
    ExitRuntime,
    OpenAudioSettings,
    OpenDisplaySettings,
    OpenGraphicsSettings,
    SaveGame { slot: u8 },
    LoadGame { slot: u8 },
    EmitEvent { event_id: String },
}

#[cfg(test)]
#[path = "ui_tests.rs"]
mod tests;
