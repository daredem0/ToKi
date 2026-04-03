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
    pub max_width: Option<f32>,
}

impl UiTextBlock {
    pub fn to_text_item(&self) -> TextItem {
        let mut item = TextItem::new_screen(self.content.clone(), self.position, self.style.clone())
            .with_anchor(self.anchor)
            .with_layer(self.layer);
        if let Some(max_width) = self.max_width {
            item = item.with_max_width(max_width);
        }
        item
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiPresentationTransform {
    pub origin: glam::Vec2,
    pub geometry_scale: f32,
    pub text_scale: f32,
}

pub fn ui_presentation_transform(
    origin: glam::Vec2,
    geometry_scale: f32,
    logical_viewport_size: glam::Vec2,
    presented_viewport_size: glam::Vec2,
) -> UiPresentationTransform {
    UiPresentationTransform {
        origin,
        geometry_scale,
        text_scale: runtime_ui_text_scale(logical_viewport_size, presented_viewport_size),
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

pub fn transform_logical_ui_rect_with_transform(
    rect: UiRect,
    transform: UiPresentationTransform,
) -> UiRect {
    transform_logical_ui_rect(rect, transform.origin, transform.geometry_scale)
}

pub fn runtime_ui_text_scale(
    logical_viewport_size: glam::Vec2,
    presented_viewport_size: glam::Vec2,
) -> f32 {
    let reference_size =
        glam::Vec2::new(logical_viewport_size.x * 7.0, logical_viewport_size.y * 7.0);
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
    transform_logical_ui_composition_with_transform(
        composition,
        UiPresentationTransform {
            origin,
            geometry_scale: scale,
            text_scale: scale,
        },
    )
}

pub fn transform_logical_ui_composition_with_scales(
    composition: &UiComposition,
    origin: glam::Vec2,
    geometry_scale: f32,
    text_scale: f32,
) -> UiComposition {
    transform_logical_ui_composition_with_transform(
        composition,
        UiPresentationTransform {
            origin,
            geometry_scale,
            text_scale,
        },
    )
}

pub fn transform_logical_ui_composition_with_transform(
    composition: &UiComposition,
    transform: UiPresentationTransform,
) -> UiComposition {
    let mut transformed = composition.clone();
    for block in &mut transformed.blocks {
        block.rect = transform_logical_ui_rect_with_transform(block.rect, transform);
        block.border_thickness *= transform.geometry_scale;
        if let Some(text) = block.text.as_mut() {
            text.position = glam::Vec2::new(
                transform.origin.x + text.position.x * transform.geometry_scale,
                transform.origin.y + text.position.y * transform.geometry_scale,
            );
            text.style.size_px *= transform.text_scale;
            text.max_width = text.max_width.map(|max_width| max_width * transform.geometry_scale);
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
