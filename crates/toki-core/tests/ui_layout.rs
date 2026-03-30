use std::collections::HashMap;

use toki_core::flags::{FlagValue, GameFlags};
use toki_core::rules::TriggerContext;
use toki_core::text::{TextAnchor, TextSlant, TextWeight};
use toki_core::ui_layout::{
    UiBinding, UiBindingContext, UiController, UiLayoutAsset, UiLayoutEngine, UiProgressBinding,
    UiRequest, UiTextSegment, UiTextTemplate, UiTheme, UiTypography, UiWidgetKind, UiWidgetNode,
};
use toki_core::value_path::ValuePathContext;

fn binding_context<'a>(
    entity_manager: &'a toki_core::entity::EntityManager,
    flags: &'a GameFlags,
    overrides: &'a HashMap<String, FlagValue>,
) -> UiBindingContext<'a, 'static, 'a> {
    static TRIGGER: TriggerContext = TriggerContext {
        trigger_self: None,
        trigger_other: None,
    };
    UiBindingContext {
        value_paths: ValuePathContext {
            entity_manager,
            game_flags: flags,
            player_id: None,
            trigger_context: &TRIGGER,
        },
        binding_overrides: overrides,
        declared_flags: &[],
    }
}

#[test]
fn ui_layout_asset_json_roundtrips() {
    let layout = UiLayoutAsset {
        id: "hud".into(),
        title: "HUD".to_string(),
        startup_visible: true,
        z_order: 2,
        root: UiWidgetNode::default(),
    };

    let json = serde_json::to_string_pretty(&layout).expect("layout should serialize");
    let decoded: UiLayoutAsset = serde_json::from_str(&json).expect("layout should deserialize");
    assert_eq!(decoded.id, layout.id);
    assert_eq!(decoded.title, layout.title);
    assert!(decoded.startup_visible);
}

#[test]
fn ui_layout_engine_composes_bindings_progress_and_frames() {
    let entity_manager = toki_core::entity::EntityManager::default();
    let mut flags = GameFlags::default();
    flags.set("coins", FlagValue::Int(7));
    let overrides = HashMap::new();

    let mut root = UiWidgetNode::default();
    root.children = vec![
        UiWidgetNode {
            id: "coins".into(),
            title: "Coins".to_string(),
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
            focusable: true,
            event_id: Some("coins_clicked".to_string()),
            ..UiWidgetNode::default()
        },
        UiWidgetNode {
            id: "progress".into(),
            title: "Progress".to_string(),
            kind: UiWidgetKind::ProgressBar {
                value: UiProgressBinding::Percent {
                    percent: UiBinding::Expression {
                        expression: "50".to_string(),
                        key: None,
                    },
                },
            },
            ..UiWidgetNode::default()
        },
    ];

    let layout = UiLayoutAsset {
        id: "hud".into(),
        title: "HUD".to_string(),
        startup_visible: true,
        z_order: 0,
        root,
    };

    let output = UiLayoutEngine::compose(
        &layout,
        &UiTheme::default(),
        glam::vec2(320.0, 180.0),
        binding_context(&entity_manager, &flags, &overrides),
        None,
    );

    assert!(output.composition.blocks.iter().any(|block| block
        .text
        .as_ref()
        .is_some_and(|text| text.content.contains("Coins: 7"))));
    assert!(output
        .widget_frames
        .iter()
        .any(|frame| frame.widget_id.as_str() == "coins"));
    assert!(output
        .hitboxes
        .iter()
        .any(|hitbox| hitbox.widget_id.as_str() == "coins" && hitbox.event_id.is_some()));
    assert!(output
        .composition
        .blocks
        .iter()
        .any(|block| block.fill_color.is_some()
            && block.rect.width > 20.0
            && block.rect.height > 2.0));
}

#[test]
fn ui_controller_handles_visibility_binding_updates_and_click_events() {
    let entity_manager = toki_core::entity::EntityManager::default();
    let flags = GameFlags::default();
    let overrides = HashMap::new();

    let mut root = UiWidgetNode::default();
    root.children = vec![UiWidgetNode {
        id: "button_like".into(),
        title: "Button".to_string(),
        focusable: true,
        event_id: Some("open_inventory".to_string()),
        kind: UiWidgetKind::Label {
            content: UiTextTemplate {
                segments: vec![UiTextSegment::Binding {
                    binding: UiBinding::ValuePath {
                        path: "flags.mode".to_string(),
                        key: Some("mode".to_string()),
                    },
                }],
            },
        },
        ..UiWidgetNode::default()
    }];

    let layout = UiLayoutAsset {
        id: "hud".into(),
        title: "HUD".to_string(),
        startup_visible: false,
        z_order: 0,
        root,
    };

    let mut controller = UiController::new([layout]);
    controller.apply_request(UiRequest::ShowUi {
        ui_id: "hud".into(),
    });
    controller.apply_request(UiRequest::UpdateUiBinding {
        ui_id: "hud".into(),
        binding_key: "mode".to_string(),
        value: FlagValue::String("Inventory".to_string()),
    });

    let rendered = controller.render_visible_surfaces(
        &UiTheme::default(),
        glam::vec2(320.0, 180.0),
        binding_context(&entity_manager, &flags, &overrides),
    );
    assert_eq!(rendered.len(), 1);
    assert!(rendered[0]
        .output
        .composition
        .blocks
        .iter()
        .any(|block| block
            .text
            .as_ref()
            .is_some_and(|text| text.content.contains("Inventory"))));

    let click_position = rendered[0]
        .output
        .hitboxes
        .iter()
        .find(|hitbox| hitbox.widget_id.as_str() == "button_like")
        .map(|hitbox| glam::vec2(hitbox.rect.x + 2.0, hitbox.rect.y + 2.0))
        .expect("button-like hitbox should exist");

    assert!(controller.handle_pointer_click(
        &UiTheme::default(),
        glam::vec2(320.0, 180.0),
        click_position,
        binding_context(&entity_manager, &flags, &overrides),
    ));
    assert_eq!(
        controller.take_emitted_events(),
        vec!["open_inventory".to_string()]
    );

    controller.apply_request(UiRequest::HideUi {
        ui_id: "hud".into(),
    });
    assert!(controller
        .render_visible_surfaces(
            &UiTheme::default(),
            glam::vec2(320.0, 180.0),
            binding_context(&entity_manager, &flags, &overrides),
        )
        .is_empty());
}

#[test]
fn root_level_widgets_keep_independent_positions() {
    let entity_manager = toki_core::entity::EntityManager::default();
    let flags = GameFlags::default();
    let overrides = HashMap::new();

    let mut root = UiWidgetNode::default();
    root.children = vec![
        UiWidgetNode {
            id: "label".into(),
            title: "Label".to_string(),
            layout: toki_core::ui_layout::UiLayoutSpec {
                anchor: toki_core::ui_layout::UiAnchor::TopLeft,
                offset: [0.0, 0.0],
                size: [80.0, 60.0],
                ..Default::default()
            },
            kind: UiWidgetKind::Label {
                content: UiTextTemplate {
                    segments: vec![UiTextSegment::Literal {
                        text: "Top Left".to_string(),
                    }],
                },
            },
            ..UiWidgetNode::default()
        },
        UiWidgetNode {
            id: "progress".into(),
            title: "Progress".to_string(),
            layout: toki_core::ui_layout::UiLayoutSpec {
                anchor: toki_core::ui_layout::UiAnchor::TopRight,
                offset: [0.0, 0.0],
                size: [40.0, 12.0],
                ..Default::default()
            },
            kind: UiWidgetKind::ProgressBar {
                value: UiProgressBinding::Percent {
                    percent: UiBinding::Expression {
                        expression: "50".to_string(),
                        key: None,
                    },
                },
            },
            ..UiWidgetNode::default()
        },
    ];

    let layout = UiLayoutAsset {
        id: "hud".into(),
        title: "HUD".to_string(),
        startup_visible: true,
        z_order: 0,
        root,
    };

    let output = UiLayoutEngine::compose(
        &layout,
        &UiTheme::default(),
        glam::vec2(160.0, 144.0),
        binding_context(&entity_manager, &flags, &overrides),
        None,
    );

    let label_frame = output
        .widget_frames
        .iter()
        .find(|frame| frame.widget_id.as_str() == "label")
        .expect("label frame should exist");
    let progress_frame = output
        .widget_frames
        .iter()
        .find(|frame| frame.widget_id.as_str() == "progress")
        .expect("progress frame should exist");

    assert!(label_frame.rect.y <= 12.0);
    assert!(progress_frame.rect.y <= 12.0);
    assert!(progress_frame.rect.x > label_frame.rect.x + label_frame.rect.width);
}

#[test]
fn label_text_defaults_to_center_and_honors_typography_overrides() {
    let entity_manager = toki_core::entity::EntityManager::default();
    let flags = GameFlags::default();
    let overrides = HashMap::new();

    let mut root = UiWidgetNode::default();
    root.children = vec![UiWidgetNode {
        id: "headline".into(),
        title: "Headline".to_string(),
        style: toki_core::ui_layout::UiWidgetStyle {
            typography: UiTypography {
                font_family: Some("Monospace".to_string()),
                font_size_px: Some(20),
                weight: Some(TextWeight::Bold),
                slant: Some(TextSlant::Italic),
                anchor: Some(TextAnchor::Center),
            },
            ..Default::default()
        },
        kind: UiWidgetKind::Label {
            content: UiTextTemplate {
                segments: vec![UiTextSegment::Literal {
                    text: "Centered".to_string(),
                }],
            },
        },
        ..UiWidgetNode::default()
    }];

    let layout = UiLayoutAsset {
        id: "hud".into(),
        title: "HUD".to_string(),
        startup_visible: true,
        z_order: 0,
        root,
    };

    let output = UiLayoutEngine::compose(
        &layout,
        &UiTheme::default(),
        glam::vec2(320.0, 180.0),
        binding_context(&entity_manager, &flags, &overrides),
        None,
    );

    let text = output
        .composition
        .blocks
        .iter()
        .find_map(|block| block.text.as_ref())
        .expect("label block should contain text");
    assert_eq!(text.anchor, TextAnchor::Center);
    assert_eq!(text.style.font_family, "Monospace");
    assert_eq!(text.style.size_px, 20.0);
    assert_eq!(text.style.weight, TextWeight::Bold);
    assert_eq!(text.style.slant, TextSlant::Italic);

    let label_frame = output
        .widget_frames
        .iter()
        .find(|frame| frame.widget_id.as_str() == "headline")
        .expect("label frame should exist");
    assert_eq!(text.position.x, label_frame.rect.center_x());
    assert_eq!(text.position.y, label_frame.rect.center_y());
}
