use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::expression::{Expression, ExpressionError};
use crate::flags::FlagValue;
use crate::ids::{UiLayoutId, UiWidgetId};
use crate::project_runtime::ProjectFlagDefinition;
use crate::text::{TextAnchor, TextSlant, TextStyle, TextWeight};
use crate::ui::{UiBlock, UiComposition, UiRect, UiTextBlock};
use crate::value_path::{ResolvedValue, ValuePathContext};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UiLayoutAsset {
    pub id: UiLayoutId,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub startup_visible: bool,
    #[serde(default)]
    pub z_order: i32,
    #[serde(default)]
    pub root: UiWidgetNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiWidgetNode {
    #[serde(default)]
    pub id: UiWidgetId,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub layout: UiLayoutSpec,
    #[serde(default)]
    pub style: UiWidgetStyle,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub focusable: bool,
    #[serde(default)]
    pub visible_if: Option<String>,
    #[serde(default)]
    pub enabled_if: Option<String>,
    #[serde(default)]
    pub kind: UiWidgetKind,
    #[serde(default)]
    pub children: Vec<UiWidgetNode>,
}

impl Default for UiWidgetNode {
    fn default() -> Self {
        Self {
            id: UiWidgetId::new("root"),
            title: "Root".to_string(),
            layout: UiLayoutSpec {
                anchor: UiAnchor::Stretch,
                size: [160.0, 144.0],
                ..UiLayoutSpec::default()
            },
            style: UiWidgetStyle::default(),
            event_id: None,
            focusable: false,
            visible_if: None,
            enabled_if: None,
            kind: UiWidgetKind::GridContainer {
                columns: 1,
                spacing: UiSpacing::default(),
            },
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiWidgetKind {
    Label {
        content: UiTextTemplate,
    },
    Image {
        image_id: String,
    },
    ProgressBar {
        value: UiProgressBinding,
    },
    GridContainer {
        columns: u16,
        spacing: UiSpacing,
    },
    ScrollList {
        collection: UiCollectionBinding,
        row_height: u16,
        row_spacing: u16,
        row_template: UiCollectionRowTemplate,
    },
}

impl Default for UiWidgetKind {
    fn default() -> Self {
        Self::Label {
            content: UiTextTemplate {
                segments: vec![UiTextSegment::Literal {
                    text: "Label".to_string(),
                }],
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UiTextTemplate {
    #[serde(default)]
    pub segments: Vec<UiTextSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiTextSegment {
    Literal { text: String },
    Binding { binding: UiBinding },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiBinding {
    ValuePath {
        path: String,
        #[serde(default)]
        key: Option<String>,
    },
    Expression {
        expression: String,
        #[serde(default)]
        key: Option<String>,
    },
}

impl UiBinding {
    pub fn key(&self) -> Option<&str> {
        match self {
            Self::ValuePath { key, .. } | Self::Expression { key, .. } => key.as_deref(),
        }
    }

    pub fn resolve(
        &self,
        context: UiBindingContext<'_, '_, '_>,
    ) -> Result<ResolvedValue, ExpressionError> {
        if let Some(key) = self.key() {
            if let Some(value) = context.binding_overrides.get(key) {
                return Ok(flag_value_to_resolved(value.clone()));
            }
        }

        match self {
            Self::ValuePath { path, .. } => crate::value_path::ValuePath::parse(path)
                .map_err(ExpressionError::from)?
                .resolve(context.value_paths)
                .map_err(ExpressionError::from),
            Self::Expression { expression, .. } => {
                Expression::parse(expression)?.evaluate(context.value_paths)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiCollectionBinding {
    PlayerInventory,
    DeclaredFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UiCollectionRowTemplate {
    #[serde(default)]
    pub segments: Vec<UiCollectionTextSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiCollectionTextSegment {
    Literal { text: String },
    ItemId,
    ItemCount,
    ItemValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum UiProgressBinding {
    CurrentMax { current: UiBinding, max: UiBinding },
    Percent { percent: UiBinding },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiLayoutSpec {
    #[serde(default)]
    pub anchor: UiAnchor,
    #[serde(default)]
    pub offset: [f32; 2],
    #[serde(default = "default_widget_size")]
    pub size: [f32; 2],
    #[serde(default)]
    pub margin: UiSpacing,
    #[serde(default)]
    pub padding: UiSpacing,
}

impl Default for UiLayoutSpec {
    fn default() -> Self {
        Self {
            anchor: UiAnchor::TopLeft,
            offset: [0.0, 0.0],
            size: default_widget_size(),
            margin: UiSpacing::default(),
            padding: UiSpacing::default(),
        }
    }
}

fn default_widget_size() -> [f32; 2] {
    [96.0, 24.0]
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum UiAnchor {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
    Stretch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiSpacing {
    #[serde(default)]
    pub left: u16,
    #[serde(default)]
    pub top: u16,
    #[serde(default)]
    pub right: u16,
    #[serde(default)]
    pub bottom: u16,
}

impl Default for UiSpacing {
    fn default() -> Self {
        Self {
            left: 4,
            top: 4,
            right: 4,
            bottom: 4,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UiWidgetStyle {
    #[serde(default)]
    pub fill_color: Option<[u8; 4]>,
    #[serde(default)]
    pub border_color: Option<[u8; 4]>,
    #[serde(default)]
    pub text_color: Option<[u8; 4]>,
    #[serde(default)]
    pub accent_color: Option<[u8; 4]>,
    #[serde(default)]
    pub typography: UiTypography,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UiTypography {
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_size_px: Option<u16>,
    #[serde(default)]
    pub weight: Option<TextWeight>,
    #[serde(default)]
    pub slant: Option<TextSlant>,
    #[serde(default)]
    pub anchor: Option<TextAnchor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiTheme {
    #[serde(default = "default_ui_font_family")]
    pub font_family: String,
    #[serde(default = "default_ui_font_size_px")]
    pub base_font_size_px: u16,
    #[serde(default = "default_ui_text_color")]
    pub foreground_color: [u8; 4],
    #[serde(default = "default_ui_background_color")]
    pub background_color: [u8; 4],
    #[serde(default = "default_ui_accent_color")]
    pub accent_color: [u8; 4],
    #[serde(default = "default_ui_border_color")]
    pub border_color: [u8; 4],
    #[serde(default = "default_ui_border_thickness_px")]
    pub border_thickness_px: u16,
    #[serde(default)]
    pub default_spacing: UiSpacing,
    #[serde(default = "default_ui_progress_fill_color")]
    pub progress_fill_color: [u8; 4],
    #[serde(default = "default_ui_progress_empty_color")]
    pub progress_empty_color: [u8; 4],
    #[serde(default = "default_ui_selection_color")]
    pub selection_color: [u8; 4],
}

impl Default for UiTheme {
    fn default() -> Self {
        Self {
            font_family: default_ui_font_family(),
            base_font_size_px: default_ui_font_size_px(),
            foreground_color: default_ui_text_color(),
            background_color: default_ui_background_color(),
            accent_color: default_ui_accent_color(),
            border_color: default_ui_border_color(),
            border_thickness_px: default_ui_border_thickness_px(),
            default_spacing: UiSpacing::default(),
            progress_fill_color: default_ui_progress_fill_color(),
            progress_empty_color: default_ui_progress_empty_color(),
            selection_color: default_ui_selection_color(),
        }
    }
}

fn default_ui_font_family() -> String {
    "Sans".to_string()
}

const fn default_ui_font_size_px() -> u16 {
    8
}

const fn default_ui_text_color() -> [u8; 4] {
    [240, 240, 240, 255]
}

const fn default_ui_background_color() -> [u8; 4] {
    [12, 18, 28, 224]
}

const fn default_ui_accent_color() -> [u8; 4] {
    [100, 220, 180, 255]
}

const fn default_ui_border_color() -> [u8; 4] {
    [255, 255, 255, 255]
}

const fn default_ui_border_thickness_px() -> u16 {
    1
}

const fn default_ui_progress_fill_color() -> [u8; 4] {
    [80, 220, 120, 255]
}

const fn default_ui_progress_empty_color() -> [u8; 4] {
    [32, 48, 56, 255]
}

const fn default_ui_selection_color() -> [u8; 4] {
    [255, 210, 90, 255]
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiCollectionItem {
    pub key: String,
    pub count: i32,
    pub value: String,
}

#[derive(Debug, Clone, Copy)]
pub struct UiBindingContext<'a, 'b, 'c> {
    pub value_paths: ValuePathContext<'a, 'b>,
    pub binding_overrides: &'c HashMap<String, FlagValue>,
    pub declared_flags: &'c [ProjectFlagDefinition],
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiWidgetHitbox {
    pub widget_id: UiWidgetId,
    pub rect: UiRect,
    pub enabled: bool,
    pub focusable: bool,
    pub event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiWidgetFrame {
    pub widget_id: UiWidgetId,
    pub rect: UiRect,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiLayoutOutput {
    pub composition: UiComposition,
    pub hitboxes: Vec<UiWidgetHitbox>,
    pub focus_order: Vec<UiWidgetId>,
    pub widget_frames: Vec<UiWidgetFrame>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiRenderedSurface {
    pub layout_id: UiLayoutId,
    pub z_order: i32,
    pub output: UiLayoutOutput,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiSurfaceState {
    pub visible: bool,
    pub z_order: i32,
    pub startup_visible: bool,
    pub binding_overrides: HashMap<String, FlagValue>,
    pub focused_widget_id: Option<UiWidgetId>,
    pub scroll_offsets: HashMap<UiWidgetId, f32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiRequest {
    ShowUi {
        ui_id: UiLayoutId,
    },
    HideUi {
        ui_id: UiLayoutId,
    },
    UpdateUiBinding {
        ui_id: UiLayoutId,
        binding_key: String,
        value: FlagValue,
    },
}

#[derive(Debug, Default)]
pub struct UiLayoutEngine;

impl UiLayoutEngine {
    pub fn compose(
        layout: &UiLayoutAsset,
        theme: &UiTheme,
        viewport_size: glam::Vec2,
        context: UiBindingContext<'_, '_, '_>,
        surface_state: Option<&UiSurfaceState>,
    ) -> UiLayoutOutput {
        let mut output = UiLayoutOutput::default();
        let root_rect = UiRect {
            x: 0.0,
            y: 0.0,
            width: viewport_size.x.max(1.0),
            height: viewport_size.y.max(1.0),
        };
        Self::compose_widget(
            &layout.root,
            root_rect,
            theme,
            &context,
            surface_state,
            &mut output,
        );
        output
    }

    fn compose_widget(
        widget: &UiWidgetNode,
        parent_rect: UiRect,
        theme: &UiTheme,
        context: &UiBindingContext<'_, '_, '_>,
        surface_state: Option<&UiSurfaceState>,
        output: &mut UiLayoutOutput,
    ) {
        if !evaluate_widget_gate(widget.visible_if.as_deref(), *context, true) {
            return;
        }

        let enabled = evaluate_widget_gate(widget.enabled_if.as_deref(), *context, true);
        let rect = resolve_widget_rect(&widget.layout, parent_rect);
        let content_rect = inset_rect_for_content(rect, widget.layout.padding);
        output.widget_frames.push(UiWidgetFrame {
            widget_id: widget.id.clone(),
            rect,
            enabled,
        });
        let highlighted = surface_state
            .and_then(|state| state.focused_widget_id.as_ref())
            .is_some_and(|focused_id| focused_id == &widget.id);

        match &widget.kind {
            UiWidgetKind::Label { content } => {
                let text = render_text_template(content, *context);
                let anchor = effective_text_anchor(widget, TextAnchor::Center);
                let mut block = make_panel_block(widget, rect, theme, highlighted);
                block.text = Some(UiTextBlock {
                    content: text,
                    position: text_anchor_position(content_rect, anchor, 0.0),
                    anchor,
                    style: text_style_for_widget(widget, theme),
                    layer: 20,
                });
                output.composition.push(block);
            }
            UiWidgetKind::Image { image_id } => {
                let anchor = effective_text_anchor(widget, TextAnchor::Center);
                let mut block = make_panel_block(widget, rect, theme, highlighted);
                block.text = Some(UiTextBlock {
                    content: image_id.clone(),
                    position: text_anchor_position(content_rect, anchor, 0.0),
                    anchor,
                    style: text_style_for_widget(widget, theme),
                    layer: 20,
                });
                output.composition.push(block);
            }
            UiWidgetKind::ProgressBar { value } => {
                let fraction = resolve_progress_fraction(value, *context);
                let panel = make_panel_block(widget, rect, theme, highlighted);
                output.composition.push(panel);
                let fill_rect = UiRect {
                    x: content_rect.x,
                    y: content_rect.y,
                    width: content_rect.width * fraction,
                    height: content_rect.height,
                };
                output.composition.push(UiBlock {
                    rect: fill_rect,
                    fill_color: Some(color_to_f32(
                        widget
                            .style
                            .accent_color
                            .unwrap_or(theme.progress_fill_color),
                    )),
                    border_color: None,
                    border_thickness: 0.0,
                    text: None,
                });
            }
            UiWidgetKind::GridContainer { columns, spacing } => {
                if widget.id.as_str() == "root" {
                    for child in &widget.children {
                        Self::compose_widget(
                            child,
                            content_rect,
                            theme,
                            context,
                            surface_state,
                            output,
                        );
                    }
                    return;
                }

                let columns = (*columns).max(1) as usize;
                let children = &widget.children;
                let cell_width = (content_rect.width
                    - spacing.horizontal() * (columns.saturating_sub(1) as f32))
                    / columns as f32;
                let cell_height = children
                    .iter()
                    .map(|child| child.layout.size[1])
                    .fold(0.0_f32, f32::max)
                    .max(24.0);

                for (index, child) in children.iter().enumerate() {
                    let row = index / columns;
                    let col = index % columns;
                    let child_parent = UiRect {
                        x: content_rect.x + col as f32 * (cell_width + spacing.horizontal()),
                        y: content_rect.y + row as f32 * (cell_height + spacing.vertical()),
                        width: cell_width.max(1.0),
                        height: cell_height.max(1.0),
                    };
                    Self::compose_widget(
                        child,
                        child_parent,
                        theme,
                        context,
                        surface_state,
                        output,
                    );
                }
            }
            UiWidgetKind::ScrollList {
                collection,
                row_height,
                row_spacing,
                row_template,
            } => {
                let rows = resolve_collection_items(collection, *context);
                let scroll_offset = surface_state
                    .and_then(|state| state.scroll_offsets.get(&widget.id).copied())
                    .unwrap_or_default();
                let mut list_panel = make_panel_block(widget, rect, theme, highlighted);
                list_panel.text = None;
                output.composition.push(list_panel);

                for (index, item) in rows.iter().enumerate() {
                    let row_top = content_rect.y
                        + index as f32 * (*row_height as f32 + *row_spacing as f32)
                        - scroll_offset;
                    if row_top + *row_height as f32 <= content_rect.y
                        || row_top >= content_rect.y + content_rect.height
                    {
                        continue;
                    }
                    let row_rect = UiRect {
                        x: content_rect.x,
                        y: row_top,
                        width: content_rect.width,
                        height: *row_height as f32,
                    };
                    let text = render_collection_row(row_template, item);
                    let row_text_rect = UiRect {
                        x: row_rect.x + 4.0,
                        y: row_rect.y,
                        width: (row_rect.width - 8.0).max(1.0),
                        height: row_rect.height,
                    };
                    let anchor = effective_text_anchor(widget, TextAnchor::CenterLeft);
                    output.composition.push(UiBlock {
                        rect: row_rect,
                        fill_color: Some(color_to_f32(theme.progress_empty_color)),
                        border_color: Some(color_to_f32(theme.border_color)),
                        border_thickness: theme.border_thickness_px as f32,
                        text: Some(UiTextBlock {
                            content: text,
                            position: text_anchor_position(row_text_rect, anchor, 0.0),
                            anchor,
                            style: text_style_for_widget(widget, theme),
                            layer: 20,
                        }),
                    });
                    let row_widget_id = UiWidgetId::new(format!("{}::{}", widget.id, item.key));
                    let row_event_id = widget
                        .event_id
                        .as_ref()
                        .map(|event_id| format!("{event_id}:{}", item.key));
                    output.hitboxes.push(UiWidgetHitbox {
                        widget_id: row_widget_id.clone(),
                        rect: row_rect,
                        enabled,
                        focusable: widget.focusable,
                        event_id: row_event_id,
                    });
                    if widget.focusable {
                        output.focus_order.push(row_widget_id);
                    }
                }
            }
        }

        if !matches!(
            widget.kind,
            UiWidgetKind::GridContainer { .. } | UiWidgetKind::ScrollList { .. }
        ) {
            if widget.focusable || widget.event_id.is_some() {
                output.hitboxes.push(UiWidgetHitbox {
                    widget_id: widget.id.clone(),
                    rect,
                    enabled,
                    focusable: widget.focusable,
                    event_id: widget.event_id.clone(),
                });
                if widget.focusable {
                    output.focus_order.push(widget.id.clone());
                }
            }
            for child in &widget.children {
                Self::compose_widget(child, content_rect, theme, context, surface_state, output);
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct UiController {
    layouts: BTreeMap<UiLayoutId, UiLayoutAsset>,
    surfaces: BTreeMap<UiLayoutId, UiSurfaceState>,
    emitted_events: Vec<String>,
}

impl UiController {
    pub fn new(layouts: impl IntoIterator<Item = UiLayoutAsset>) -> Self {
        let mut controller = Self::default();
        for layout in layouts {
            controller.register_layout(layout);
        }
        controller
    }

    pub fn register_layout(&mut self, layout: UiLayoutAsset) {
        let surface_state = UiSurfaceState {
            visible: layout.startup_visible,
            z_order: layout.z_order,
            startup_visible: layout.startup_visible,
            binding_overrides: HashMap::new(),
            focused_widget_id: None,
            scroll_offsets: HashMap::new(),
        };
        self.surfaces.insert(layout.id.clone(), surface_state);
        self.layouts.insert(layout.id.clone(), layout);
    }

    pub fn surface_state(&self, layout_id: &UiLayoutId) -> Option<&UiSurfaceState> {
        self.surfaces.get(layout_id)
    }

    pub fn apply_request(&mut self, request: UiRequest) {
        match request {
            UiRequest::ShowUi { ui_id } => {
                if let Some(surface) = self.surfaces.get_mut(&ui_id) {
                    surface.visible = true;
                }
            }
            UiRequest::HideUi { ui_id } => {
                if let Some(surface) = self.surfaces.get_mut(&ui_id) {
                    surface.visible = false;
                    surface.focused_widget_id = None;
                }
            }
            UiRequest::UpdateUiBinding {
                ui_id,
                binding_key,
                value,
            } => {
                if let Some(surface) = self.surfaces.get_mut(&ui_id) {
                    surface.binding_overrides.insert(binding_key, value);
                }
            }
        }
    }

    pub fn render_visible_surfaces(
        &self,
        theme: &UiTheme,
        viewport_size: glam::Vec2,
        context: UiBindingContext<'_, '_, '_>,
    ) -> Vec<UiRenderedSurface> {
        let mut rendered = self
            .layouts
            .values()
            .filter_map(|layout| {
                let surface_state = self.surfaces.get(&layout.id)?;
                if !surface_state.visible {
                    return None;
                }
                let mut surface_context = context;
                surface_context.binding_overrides = &surface_state.binding_overrides;
                Some(UiRenderedSurface {
                    layout_id: layout.id.clone(),
                    z_order: surface_state.z_order,
                    output: UiLayoutEngine::compose(
                        layout,
                        theme,
                        viewport_size,
                        surface_context,
                        Some(surface_state),
                    ),
                })
            })
            .collect::<Vec<_>>();
        rendered.sort_by(|left, right| left.z_order.cmp(&right.z_order));
        rendered
    }

    pub fn handle_pointer_click(
        &mut self,
        theme: &UiTheme,
        viewport_size: glam::Vec2,
        position: glam::Vec2,
        context: UiBindingContext<'_, '_, '_>,
    ) -> bool {
        let surfaces = self.render_visible_surfaces(theme, viewport_size, context);
        for surface in surfaces.iter().rev() {
            if let Some(hit) = surface
                .output
                .hitboxes
                .iter()
                .rev()
                .find(|hitbox| hitbox.enabled && rect_contains(hitbox.rect, position))
            {
                if let Some(state) = self.surfaces.get_mut(&surface.layout_id) {
                    state.focused_widget_id = Some(hit.widget_id.clone());
                }
                if let Some(event_id) = &hit.event_id {
                    self.emitted_events.push(event_id.clone());
                }
                return true;
            }
        }
        false
    }

    pub fn focus_next(
        &mut self,
        theme: &UiTheme,
        viewport_size: glam::Vec2,
        context: UiBindingContext<'_, '_, '_>,
    ) -> bool {
        let surfaces = self.render_visible_surfaces(theme, viewport_size, context);
        for surface in surfaces.iter().rev() {
            if surface.output.focus_order.is_empty() {
                continue;
            }
            if let Some(state) = self.surfaces.get_mut(&surface.layout_id) {
                let next_index = state
                    .focused_widget_id
                    .as_ref()
                    .and_then(|current| {
                        surface
                            .output
                            .focus_order
                            .iter()
                            .position(|widget_id| widget_id == current)
                    })
                    .map(|index| (index + 1) % surface.output.focus_order.len())
                    .unwrap_or(0);
                state.focused_widget_id = Some(surface.output.focus_order[next_index].clone());
                return true;
            }
        }
        false
    }

    pub fn activate_focused(
        &mut self,
        theme: &UiTheme,
        viewport_size: glam::Vec2,
        context: UiBindingContext<'_, '_, '_>,
    ) -> bool {
        for (layout_id, surface) in &self.surfaces {
            if !surface.visible {
                continue;
            }
            let Some(focused_widget_id) = &surface.focused_widget_id else {
                continue;
            };
            let Some(layout) = self.layouts.get(layout_id) else {
                continue;
            };
            let mut surface_context = context;
            surface_context.binding_overrides = &surface.binding_overrides;
            let output = UiLayoutEngine::compose(
                layout,
                theme,
                viewport_size,
                surface_context,
                Some(surface),
            );
            if let Some(hitbox) = output
                .hitboxes
                .into_iter()
                .find(|hitbox| &hitbox.widget_id == focused_widget_id)
            {
                if let Some(event_id) = hitbox.event_id {
                    self.emitted_events.push(event_id);
                    return true;
                }
            }
        }
        false
    }

    pub fn scroll_surface_widget(
        &mut self,
        layout_id: &UiLayoutId,
        widget_id: &UiWidgetId,
        delta: f32,
    ) {
        if let Some(surface) = self.surfaces.get_mut(layout_id) {
            let entry = surface.scroll_offsets.entry(widget_id.clone()).or_default();
            *entry = (*entry + delta).max(0.0);
        }
    }

    pub fn take_emitted_events(&mut self) -> Vec<String> {
        std::mem::take(&mut self.emitted_events)
    }
}

fn resolve_widget_rect(layout: &UiLayoutSpec, parent_rect: UiRect) -> UiRect {
    let width = layout.size[0].max(1.0);
    let height = layout.size[1].max(1.0);
    let (x, y, resolved_width, resolved_height) = match layout.anchor {
        UiAnchor::TopLeft => (
            parent_rect.x + layout.offset[0],
            parent_rect.y + layout.offset[1],
            width,
            height,
        ),
        UiAnchor::TopRight => (
            parent_rect.x + parent_rect.width - width + layout.offset[0],
            parent_rect.y + layout.offset[1],
            width,
            height,
        ),
        UiAnchor::BottomLeft => (
            parent_rect.x + layout.offset[0],
            parent_rect.y + parent_rect.height - height + layout.offset[1],
            width,
            height,
        ),
        UiAnchor::BottomRight => (
            parent_rect.x + parent_rect.width - width + layout.offset[0],
            parent_rect.y + parent_rect.height - height + layout.offset[1],
            width,
            height,
        ),
        UiAnchor::Center => (
            parent_rect.x + (parent_rect.width - width) * 0.5 + layout.offset[0],
            parent_rect.y + (parent_rect.height - height) * 0.5 + layout.offset[1],
            width,
            height,
        ),
        UiAnchor::Stretch => (
            parent_rect.x + layout.offset[0],
            parent_rect.y + layout.offset[1],
            (parent_rect.width - layout.offset[0] * 2.0).max(1.0),
            (parent_rect.height - layout.offset[1] * 2.0).max(1.0),
        ),
    };
    let rect = UiRect {
        x,
        y,
        width: resolved_width,
        height: resolved_height,
    };
    inset_rect(rect, layout.margin)
}

fn inset_rect(rect: UiRect, spacing: UiSpacing) -> UiRect {
    let x = rect.x + spacing.left as f32;
    let y = rect.y + spacing.top as f32;
    let width = (rect.width - spacing.left as f32 - spacing.right as f32).max(0.0);
    let height = (rect.height - spacing.top as f32 - spacing.bottom as f32).max(0.0);
    UiRect {
        x,
        y,
        width,
        height,
    }
}

fn inset_rect_for_content(rect: UiRect, spacing: UiSpacing) -> UiRect {
    let inset = inset_rect(rect, spacing);
    if inset.width <= 0.0 || inset.height <= 0.0 {
        rect
    } else {
        inset
    }
}

fn make_panel_block(
    widget: &UiWidgetNode,
    rect: UiRect,
    theme: &UiTheme,
    highlighted: bool,
) -> UiBlock {
    UiBlock {
        rect,
        fill_color: Some(color_to_f32(
            widget.style.fill_color.unwrap_or(theme.background_color),
        )),
        border_color: Some(color_to_f32(if highlighted {
            theme.selection_color
        } else {
            widget.style.border_color.unwrap_or(theme.border_color)
        })),
        border_thickness: theme.border_thickness_px.max(1) as f32,
        text: None,
    }
}

fn text_style_for_widget(widget: &UiWidgetNode, theme: &UiTheme) -> TextStyle {
    TextStyle {
        font_family: widget
            .style
            .typography
            .font_family
            .clone()
            .unwrap_or_else(|| theme.font_family.clone()),
        size_px: widget
            .style
            .typography
            .font_size_px
            .map(f32::from)
            .unwrap_or(theme.base_font_size_px as f32),
        weight: widget.style.typography.weight.unwrap_or(TextWeight::Normal),
        slant: widget.style.typography.slant.unwrap_or(TextSlant::Normal),
        color: color_to_f32(widget.style.text_color.unwrap_or(theme.foreground_color)),
    }
}

fn effective_text_anchor(widget: &UiWidgetNode, default_anchor: TextAnchor) -> TextAnchor {
    widget.style.typography.anchor.unwrap_or(default_anchor)
}

fn text_anchor_position(rect: UiRect, anchor: TextAnchor, inset: f32) -> glam::Vec2 {
    let left = rect.x + inset;
    let right = rect.x + rect.width - inset;
    let top = rect.y + inset;
    let bottom = rect.y + rect.height - inset;
    let center_x = rect.center_x();
    let center_y = rect.center_y();
    match anchor {
        TextAnchor::TopLeft => glam::vec2(left, top),
        TextAnchor::TopCenter => glam::vec2(center_x, top),
        TextAnchor::TopRight => glam::vec2(right, top),
        TextAnchor::CenterLeft => glam::vec2(left, center_y),
        TextAnchor::Center => glam::vec2(center_x, center_y),
        TextAnchor::CenterRight => glam::vec2(right, center_y),
        TextAnchor::BottomLeft => glam::vec2(left, bottom),
        TextAnchor::BottomCenter => glam::vec2(center_x, bottom),
        TextAnchor::BottomRight => glam::vec2(right, bottom),
    }
}

fn render_text_template(
    template: &UiTextTemplate,
    context: UiBindingContext<'_, '_, '_>,
) -> String {
    template
        .segments
        .iter()
        .map(|segment| match segment {
            UiTextSegment::Literal { text } => text.clone(),
            UiTextSegment::Binding { binding } => binding
                .resolve(context)
                .map(resolved_value_to_string)
                .unwrap_or_else(|error| format!("{{{error}}}")),
        })
        .collect::<Vec<_>>()
        .join("")
}

fn render_collection_row(template: &UiCollectionRowTemplate, item: &UiCollectionItem) -> String {
    template
        .segments
        .iter()
        .map(|segment| match segment {
            UiCollectionTextSegment::Literal { text } => text.clone(),
            UiCollectionTextSegment::ItemId => item.key.clone(),
            UiCollectionTextSegment::ItemCount => item.count.to_string(),
            UiCollectionTextSegment::ItemValue => item.value.clone(),
        })
        .collect::<Vec<_>>()
        .join("")
}

fn resolve_progress_fraction(
    binding: &UiProgressBinding,
    context: UiBindingContext<'_, '_, '_>,
) -> f32 {
    match binding {
        UiProgressBinding::CurrentMax { current, max } => {
            let current = current
                .resolve(context)
                .ok()
                .and_then(value_as_int)
                .unwrap_or_default();
            let max = max
                .resolve(context)
                .ok()
                .and_then(value_as_int)
                .unwrap_or(1);
            if max <= 0 {
                0.0
            } else {
                (current as f32 / max as f32).clamp(0.0, 1.0)
            }
        }
        UiProgressBinding::Percent { percent } => percent
            .resolve(context)
            .ok()
            .and_then(value_as_int)
            .map(|value| (value as f32 / 100.0).clamp(0.0, 1.0))
            .unwrap_or_default(),
    }
}

fn resolve_collection_items(
    binding: &UiCollectionBinding,
    context: UiBindingContext<'_, '_, '_>,
) -> Vec<UiCollectionItem> {
    match binding {
        UiCollectionBinding::PlayerInventory => context
            .value_paths
            .player_id
            .and_then(|player_id| {
                context
                    .value_paths
                    .entity_manager
                    .storage()
                    .components()
                    .inventory(player_id)
            })
            .map(|inventory| {
                let mut items = inventory
                    .items
                    .iter()
                    .map(|(item_id, count)| UiCollectionItem {
                        key: item_id.clone(),
                        count: *count as i32,
                        value: count.to_string(),
                    })
                    .collect::<Vec<_>>();
                items.sort_by(|left, right| left.key.cmp(&right.key));
                items
            })
            .unwrap_or_default(),
        UiCollectionBinding::DeclaredFlags => context
            .declared_flags
            .iter()
            .map(|flag| {
                let value = context
                    .value_paths
                    .game_flags
                    .get(&flag.id)
                    .cloned()
                    .unwrap_or_else(|| flag.default_value.clone());
                UiCollectionItem {
                    key: flag.id.clone(),
                    count: value.as_int().unwrap_or_default(),
                    value: match value {
                        FlagValue::Bool(value) => value.to_string(),
                        FlagValue::Int(value) => value.to_string(),
                        FlagValue::String(value) => value,
                    },
                }
            })
            .collect(),
    }
}

fn evaluate_widget_gate(
    expression: Option<&str>,
    context: UiBindingContext<'_, '_, '_>,
    default_value: bool,
) -> bool {
    let Some(expression) = expression else {
        return default_value;
    };
    match Expression::parse(expression).and_then(|expr| expr.evaluate(context.value_paths)) {
        Ok(ResolvedValue::Bool(value)) => value,
        Ok(_) | Err(_) => default_value,
    }
}

fn color_to_f32(color: [u8; 4]) -> [f32; 4] {
    [
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
        color[3] as f32 / 255.0,
    ]
}

fn flag_value_to_resolved(value: FlagValue) -> ResolvedValue {
    match value {
        FlagValue::Bool(value) => ResolvedValue::Bool(value),
        FlagValue::Int(value) => ResolvedValue::Int(value),
        FlagValue::String(value) => ResolvedValue::String(value),
    }
}

fn resolved_value_to_string(value: ResolvedValue) -> String {
    match value {
        ResolvedValue::Bool(value) => value.to_string(),
        ResolvedValue::Int(value) => value.to_string(),
        ResolvedValue::String(value) => value,
    }
}

fn value_as_int(value: ResolvedValue) -> Option<i32> {
    match value {
        ResolvedValue::Int(value) => Some(value),
        _ => None,
    }
}

fn rect_contains(rect: UiRect, position: glam::Vec2) -> bool {
    position.x >= rect.x
        && position.x <= rect.x + rect.width
        && position.y >= rect.y
        && position.y <= rect.y + rect.height
}

impl UiSpacing {
    fn horizontal(self) -> f32 {
        self.left as f32 + self.right as f32
    }

    fn vertical(self) -> f32 {
        self.top as f32 + self.bottom as f32
    }
}

#[cfg(test)]
mod tests {
    use super::{
        UiAnchor, UiBinding, UiBindingContext, UiCollectionBinding, UiCollectionRowTemplate,
        UiCollectionTextSegment, UiController, UiLayoutAsset, UiLayoutEngine, UiLayoutSpec,
        UiProgressBinding, UiRequest, UiTextSegment, UiTextTemplate, UiTheme, UiWidgetKind,
        UiWidgetNode, UiWidgetStyle,
    };
    use crate::entity::EntityManager;
    use crate::flags::{FlagValue, GameFlags};
    use crate::ids::{UiLayoutId, UiWidgetId};
    use crate::project_runtime::ProjectFlagDefinition;
    use crate::rules::TriggerContext;
    use crate::value_path::ValuePathContext;
    use std::collections::HashMap;

    fn binding_context<'a>(
        entity_manager: &'a EntityManager,
        game_flags: &'a GameFlags,
        player_id: Option<crate::entity::EntityId>,
        binding_overrides: &'a HashMap<String, FlagValue>,
        declared_flags: &'a [ProjectFlagDefinition],
    ) -> UiBindingContext<'a, 'a, 'a> {
        const EMPTY_TRIGGER_CONTEXT: TriggerContext = TriggerContext {
            trigger_self: None,
            trigger_other: None,
        };
        UiBindingContext {
            value_paths: ValuePathContext {
                entity_manager,
                game_flags,
                player_id,
                trigger_context: &EMPTY_TRIGGER_CONTEXT,
            },
            binding_overrides,
            declared_flags,
        }
    }

    #[test]
    fn ui_layout_asset_round_trips_through_json() {
        let asset = UiLayoutAsset {
            id: UiLayoutId::new("hud"),
            title: "HUD".to_string(),
            startup_visible: true,
            z_order: 10,
            root: UiWidgetNode {
                id: UiWidgetId::new("root"),
                title: "Root".to_string(),
                kind: UiWidgetKind::Label {
                    content: UiTextTemplate {
                        segments: vec![
                            UiTextSegment::Literal {
                                text: "Coins: ".to_string(),
                            },
                            UiTextSegment::Binding {
                                binding: UiBinding::ValuePath {
                                    path: "flags.coins".to_string(),
                                    key: None,
                                },
                            },
                        ],
                    },
                },
                ..UiWidgetNode::default()
            },
        };

        let json = serde_json::to_string_pretty(&asset).expect("asset should serialize");
        let decoded = serde_json::from_str::<UiLayoutAsset>(&json).expect("asset should decode");
        assert_eq!(decoded, asset);
    }

    #[test]
    fn layout_engine_positions_widgets_using_anchor_padding_and_progress_fill() {
        let entity_manager = EntityManager::new();
        let mut flags = GameFlags::default();
        flags.set("health", FlagValue::Int(25));
        flags.set("health_max", FlagValue::Int(100));
        let layout = UiLayoutAsset {
            id: UiLayoutId::new("hud"),
            root: UiWidgetNode {
                id: UiWidgetId::new("root"),
                kind: UiWidgetKind::GridContainer {
                    columns: 1,
                    spacing: super::UiSpacing::default(),
                },
                children: vec![
                    UiWidgetNode {
                        id: UiWidgetId::new("label"),
                        kind: UiWidgetKind::Label {
                            content: UiTextTemplate {
                                segments: vec![
                                    UiTextSegment::Literal {
                                        text: "HP ".to_string(),
                                    },
                                    UiTextSegment::Binding {
                                        binding: UiBinding::ValuePath {
                                            path: "flags.health".to_string(),
                                            key: None,
                                        },
                                    },
                                ],
                            },
                        },
                        layout: UiLayoutSpec {
                            anchor: UiAnchor::TopLeft,
                            offset: [4.0, 6.0],
                            size: [80.0, 18.0],
                            ..UiLayoutSpec::default()
                        },
                        ..UiWidgetNode::default()
                    },
                    UiWidgetNode {
                        id: UiWidgetId::new("bar"),
                        kind: UiWidgetKind::ProgressBar {
                            value: UiProgressBinding::CurrentMax {
                                current: UiBinding::ValuePath {
                                    path: "flags.health".to_string(),
                                    key: None,
                                },
                                max: UiBinding::ValuePath {
                                    path: "flags.health_max".to_string(),
                                    key: None,
                                },
                            },
                        },
                        layout: UiLayoutSpec {
                            anchor: UiAnchor::TopLeft,
                            offset: [4.0, 26.0],
                            size: [100.0, 12.0],
                            ..UiLayoutSpec::default()
                        },
                        ..UiWidgetNode::default()
                    },
                ],
                ..UiWidgetNode::default()
            },
            ..UiLayoutAsset::default()
        };

        let output = UiLayoutEngine::compose(
            &layout,
            &UiTheme::default(),
            glam::Vec2::new(160.0, 144.0),
            binding_context(&entity_manager, &flags, None, &HashMap::new(), &[]),
            None,
        );

        assert!(output.composition.blocks.iter().any(|block| block
            .text
            .as_ref()
            .is_some_and(|text| text.content == "HP 25")));
        assert!(output
            .composition
            .blocks
            .iter()
            .any(|block| (block.rect.width - 23.0).abs() <= 1.5));
    }

    #[test]
    fn layout_engine_renders_inventory_scroll_list_rows() {
        let mut entity_manager = EntityManager::new();
        let mut player_attributes = crate::entity::EntityAttributes::default();
        player_attributes.behavior.has_inventory = true;
        let player_id = entity_manager.spawn_entity(
            crate::entity::EntityKind::Player,
            glam::IVec2::new(0, 0),
            glam::UVec2::new(16, 16),
            player_attributes,
        );
        entity_manager
            .storage_mut()
            .components_mut()
            .ensure_inventory(player_id)
            .add_item("potion", 3);
        entity_manager
            .storage_mut()
            .components_mut()
            .ensure_inventory(player_id)
            .add_item("rope", 1);
        let flags = GameFlags::default();
        let layout = UiLayoutAsset {
            id: UiLayoutId::new("inventory"),
            root: UiWidgetNode {
                id: UiWidgetId::new("root"),
                kind: UiWidgetKind::ScrollList {
                    collection: UiCollectionBinding::PlayerInventory,
                    row_height: 14,
                    row_spacing: 2,
                    row_template: UiCollectionRowTemplate {
                        segments: vec![
                            UiCollectionTextSegment::ItemId,
                            UiCollectionTextSegment::Literal {
                                text: " x".to_string(),
                            },
                            UiCollectionTextSegment::ItemCount,
                        ],
                    },
                },
                event_id: Some("inventory_pick".to_string()),
                focusable: true,
                layout: UiLayoutSpec {
                    anchor: UiAnchor::TopLeft,
                    size: [120.0, 40.0],
                    ..UiLayoutSpec::default()
                },
                ..UiWidgetNode::default()
            },
            ..UiLayoutAsset::default()
        };

        let output = UiLayoutEngine::compose(
            &layout,
            &UiTheme::default(),
            glam::Vec2::new(160.0, 144.0),
            binding_context(
                &entity_manager,
                &flags,
                Some(player_id),
                &HashMap::new(),
                &[],
            ),
            None,
        );

        assert!(output.composition.blocks.iter().any(|block| block
            .text
            .as_ref()
            .is_some_and(|text| text.content == "potion x3")));
        assert!(output
            .hitboxes
            .iter()
            .any(|hitbox| hitbox.event_id.as_deref() == Some("inventory_pick:potion")));
    }

    #[test]
    fn widget_visibility_and_binding_overrides_are_applied() {
        let entity_manager = EntityManager::new();
        let mut flags = GameFlags::default();
        flags.set("show", FlagValue::Bool(false));
        let layout = UiLayoutAsset {
            id: UiLayoutId::new("hud"),
            root: UiWidgetNode {
                id: UiWidgetId::new("root"),
                kind: UiWidgetKind::Label {
                    content: UiTextTemplate {
                        segments: vec![UiTextSegment::Binding {
                            binding: UiBinding::ValuePath {
                                path: "flags.score".to_string(),
                                key: Some("score".to_string()),
                            },
                        }],
                    },
                },
                visible_if: Some("flags.show".to_string()),
                ..UiWidgetNode::default()
            },
            ..UiLayoutAsset::default()
        };
        let mut controller = UiController::new([layout]);
        controller.apply_request(UiRequest::ShowUi {
            ui_id: UiLayoutId::new("hud"),
        });
        controller.apply_request(UiRequest::UpdateUiBinding {
            ui_id: UiLayoutId::new("hud"),
            binding_key: "score".to_string(),
            value: FlagValue::Int(77),
        });

        let hidden = controller.render_visible_surfaces(
            &UiTheme::default(),
            glam::Vec2::new(160.0, 144.0),
            binding_context(&entity_manager, &flags, None, &HashMap::new(), &[]),
        );
        assert_eq!(hidden.len(), 1);
        assert!(hidden[0].output.composition.blocks.is_empty());

        flags.set("show", FlagValue::Bool(true));
        let visible = controller.render_visible_surfaces(
            &UiTheme::default(),
            glam::Vec2::new(160.0, 144.0),
            binding_context(&entity_manager, &flags, None, &HashMap::new(), &[]),
        );
        assert!(visible[0]
            .output
            .composition
            .blocks
            .iter()
            .any(|block| block.text.as_ref().is_some_and(|text| text.content == "77")));
    }

    #[test]
    fn controller_can_focus_and_emit_widget_events() {
        let entity_manager = EntityManager::new();
        let flags = GameFlags::default();
        let layout = UiLayoutAsset {
            id: UiLayoutId::new("hud"),
            startup_visible: true,
            root: UiWidgetNode {
                id: UiWidgetId::new("action"),
                kind: UiWidgetKind::Label {
                    content: UiTextTemplate {
                        segments: vec![UiTextSegment::Literal {
                            text: "Continue".to_string(),
                        }],
                    },
                },
                focusable: true,
                event_id: Some("continue".to_string()),
                layout: UiLayoutSpec {
                    anchor: UiAnchor::TopLeft,
                    size: [80.0, 20.0],
                    ..UiLayoutSpec::default()
                },
                style: UiWidgetStyle {
                    fill_color: Some([40, 52, 60, 255]),
                    ..UiWidgetStyle::default()
                },
                ..UiWidgetNode::default()
            },
            ..UiLayoutAsset::default()
        };
        let mut controller = UiController::new([layout]);

        assert!(controller.handle_pointer_click(
            &UiTheme::default(),
            glam::Vec2::new(160.0, 144.0),
            glam::Vec2::new(10.0, 10.0),
            binding_context(&entity_manager, &flags, None, &HashMap::new(), &[]),
        ));
        assert_eq!(
            controller.take_emitted_events(),
            vec!["continue".to_string()]
        );

        assert!(controller.focus_next(
            &UiTheme::default(),
            glam::Vec2::new(160.0, 144.0),
            binding_context(&entity_manager, &flags, None, &HashMap::new(), &[]),
        ));
    }

    #[test]
    fn declared_flags_collection_uses_default_values() {
        let entity_manager = EntityManager::new();
        let flags = GameFlags::default();
        let declared_flags = vec![ProjectFlagDefinition {
            id: "quest_state".to_string(),
            default_value: FlagValue::String("new".to_string()),
        }];
        let layout = UiLayoutAsset {
            id: UiLayoutId::new("debug"),
            root: UiWidgetNode {
                id: UiWidgetId::new("flags"),
                kind: UiWidgetKind::ScrollList {
                    collection: UiCollectionBinding::DeclaredFlags,
                    row_height: 14,
                    row_spacing: 0,
                    row_template: UiCollectionRowTemplate {
                        segments: vec![
                            UiCollectionTextSegment::ItemId,
                            UiCollectionTextSegment::Literal {
                                text: "=".to_string(),
                            },
                            UiCollectionTextSegment::ItemValue,
                        ],
                    },
                },
                ..UiWidgetNode::default()
            },
            ..UiLayoutAsset::default()
        };
        let output = UiLayoutEngine::compose(
            &layout,
            &UiTheme::default(),
            glam::Vec2::new(160.0, 144.0),
            binding_context(
                &entity_manager,
                &flags,
                None,
                &HashMap::new(),
                &declared_flags,
            ),
            None,
        );
        assert!(output.composition.blocks.iter().any(|block| block
            .text
            .as_ref()
            .is_some_and(|text| text.content == "quest_state=new")));
    }
}
