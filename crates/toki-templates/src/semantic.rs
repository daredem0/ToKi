use serde::{Deserialize, Serialize};

pub const TEMPLATE_SEMANTIC_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateTargetDomain {
    Rules,
    MenusDialogs,
    EntityDefinitions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttackMode {
    Melee,
    Projectile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TemplateSurfaceAction {
    CloseUi,
    CloseSurface,
    OpenSurface { surface_id: String },
    Back,
    ExitRuntime,
    EmitEvent { event_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateSemanticPlan {
    #[serde(default = "default_semantic_version")]
    pub semantic_version: u32,
    #[serde(default)]
    pub items: Vec<TemplateSemanticItem>,
}

const fn default_semantic_version() -> u32 {
    TEMPLATE_SEMANTIC_VERSION
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TemplateSemanticItem {
    CreateAttackBehavior {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        actor_entity_definition_id: Option<String>,
        trigger_input_action: String,
        mode: AttackMode,
        cooldown_ticks: u32,
        damage: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        animation_state: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        projectile_entity_definition_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sound_id: Option<String>,
    },
    CreatePickupBehavior {
        id: String,
        pickup_entity_definition_id: String,
        collector_entity_definition_id: String,
        item_id: String,
        count: u32,
    },
    CreateProjectileBehavior {
        id: String,
        projectile_entity_definition_id: String,
        damage: u32,
        speed: u32,
        lifetime_ticks: u32,
    },
    CreateConfirmationDialog {
        id: String,
        title: String,
        body: String,
        confirm_label: String,
        cancel_label: String,
        confirm_action: TemplateSurfaceAction,
        cancel_action: TemplateSurfaceAction,
    },
    CreatePauseMenuFlow {
        id: String,
        pause_surface_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        inventory_surface_id: Option<String>,
        include_resume: bool,
        include_inventory: bool,
        include_exit: bool,
        include_exit_confirmation_dialog: bool,
    },
    ConfigureEntityCapability {
        id: String,
        entity_definition_id: String,
        capability_id: String,
    },
}

impl TemplateSemanticItem {
    pub fn target_domains(&self) -> Vec<TemplateTargetDomain> {
        match self {
            TemplateSemanticItem::CreateAttackBehavior { .. }
            | TemplateSemanticItem::CreatePickupBehavior { .. }
            | TemplateSemanticItem::CreateProjectileBehavior { .. } => {
                vec![
                    TemplateTargetDomain::Rules,
                    TemplateTargetDomain::EntityDefinitions,
                ]
            }
            TemplateSemanticItem::CreateConfirmationDialog { .. }
            | TemplateSemanticItem::CreatePauseMenuFlow { .. } => {
                vec![TemplateTargetDomain::MenusDialogs]
            }
            TemplateSemanticItem::ConfigureEntityCapability { .. } => {
                vec![TemplateTargetDomain::EntityDefinitions]
            }
        }
    }
}
