use toki_core::entity::{EntityDefinition, PrimaryActionDef, PrimaryActionMode};
use toki_templates::{
    AttackMode, TemplateSemanticItem, TemplateSemanticPlan, TEMPLATE_SEMANTIC_VERSION,
};

use crate::{TemplateLoweringError, TemplateLoweringErrorCode};

pub trait EntityDefinitionResolver {
    fn load_entity_definition(
        &self,
        entity_definition_id: &str,
    ) -> Result<Option<EntityDefinition>, TemplateLoweringError>;
}

#[derive(Debug, Clone)]
pub enum LoweredTemplateOperation {
    UpsertEntityDefinition {
        entity_definition_id: String,
        definition: EntityDefinition,
    },
}

#[derive(Debug, Clone)]
pub struct LoweredTemplatePlan {
    pub operations: Vec<LoweredTemplateOperation>,
}

#[derive(Debug, Default)]
pub struct TemplateLowerer;

impl TemplateLowerer {
    pub fn new() -> Self {
        Self
    }

    pub fn lower_plan<R: EntityDefinitionResolver>(
        &self,
        plan: &TemplateSemanticPlan,
        resolver: &R,
    ) -> Result<LoweredTemplatePlan, TemplateLoweringError> {
        if plan.semantic_version != TEMPLATE_SEMANTIC_VERSION {
            return Err(TemplateLoweringError::new(
                TemplateLoweringErrorCode::UnsupportedSemanticVersion,
                format!(
                    "unsupported template semantic version {}",
                    plan.semantic_version
                ),
            ));
        }

        let mut operations = Vec::new();
        for item in &plan.items {
            match item {
                TemplateSemanticItem::CreateAttackBehavior {
                    actor_entity_definition_id,
                    trigger_input_action,
                    mode,
                    cooldown_ticks,
                    damage,
                    animation_state,
                    projectile_entity_definition_id,
                    sound_id,
                    ..
                } => {
                    let actor_entity_definition_id =
                        actor_entity_definition_id.as_ref().ok_or_else(|| {
                            TemplateLoweringError::new(
                                TemplateLoweringErrorCode::InvalidLoweringTarget,
                                "attack behavior lowering requires an actor_entity_definition_id",
                            )
                        })?;

                    if trigger_input_action != "attack_primary" {
                        return Err(TemplateLoweringError::new(
                            TemplateLoweringErrorCode::UnsupportedSemanticItem,
                            format!(
                                "attack behavior lowering does not support trigger_input_action '{}'",
                                trigger_input_action
                            ),
                        ));
                    }

                    if matches!(mode, AttackMode::Projectile) {
                        return Err(TemplateLoweringError::new(
                            TemplateLoweringErrorCode::UnsupportedSemanticItem,
                            format!(
                                "projectile attack lowering for actor '{}' is not implemented until projectile behavior lowering exists (requested projectile {:?})",
                                actor_entity_definition_id, projectile_entity_definition_id
                            ),
                        ));
                    }

                    let mut actor_definition = resolver
                        .load_entity_definition(actor_entity_definition_id)?
                        .ok_or_else(|| {
                            TemplateLoweringError::new(
                                TemplateLoweringErrorCode::MissingEntityDefinition,
                                format!(
                                    "entity definition '{}' was not found",
                                    actor_entity_definition_id
                                ),
                            )
                        })?;

                    if let Some(animation_state) = animation_state {
                        let has_clip = actor_definition
                            .animations
                            .clips
                            .iter()
                            .any(|clip| clip.state.eq_ignore_ascii_case(animation_state));
                        if !has_clip {
                            return Err(TemplateLoweringError::new(
                                TemplateLoweringErrorCode::InvalidLoweringTarget,
                                format!(
                                    "entity definition '{}' does not author animation state '{}'",
                                    actor_entity_definition_id, animation_state
                                ),
                            ));
                        }
                    }

                    actor_definition.attributes.primary_action = Some(PrimaryActionDef {
                        mode: PrimaryActionMode::Melee,
                        cooldown_ticks: *cooldown_ticks,
                        damage: *damage,
                        animation_state: animation_state.clone(),
                        sound_id: sound_id.clone(),
                        projectile: None,
                    });
                    actor_definition.attributes.primary_projectile = None;

                    operations.push(LoweredTemplateOperation::UpsertEntityDefinition {
                        entity_definition_id: actor_entity_definition_id.clone(),
                        definition: actor_definition,
                    });
                }
                unsupported => {
                    return Err(TemplateLoweringError::new(
                        TemplateLoweringErrorCode::UnsupportedSemanticItem,
                        format!(
                            "semantic item '{}' is not supported by this lowering slice yet",
                            semantic_item_name(unsupported)
                        ),
                    ))
                }
            }
        }

        Ok(LoweredTemplatePlan { operations })
    }
}

fn semantic_item_name(item: &TemplateSemanticItem) -> &'static str {
    match item {
        TemplateSemanticItem::CreateAttackBehavior { .. } => "create_attack_behavior",
        TemplateSemanticItem::CreatePickupBehavior { .. } => "create_pickup_behavior",
        TemplateSemanticItem::CreateProjectileBehavior { .. } => "create_projectile_behavior",
        TemplateSemanticItem::CreateConfirmationDialog { .. } => "create_confirmation_dialog",
        TemplateSemanticItem::CreatePauseMenuFlow { .. } => "create_pause_menu_flow",
        TemplateSemanticItem::ConfigureEntityCapability { .. } => "configure_entity_capability",
    }
}
