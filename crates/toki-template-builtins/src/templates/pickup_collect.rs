use std::collections::BTreeMap;

use toki_templates::{
    TemplateDescriptor, TemplateParameter, TemplateParameterKind, TemplateProviderError,
    TemplateProviderErrorCode, TemplateSemanticItem, TemplateSemanticPlan, TemplateValue,
    TEMPLATE_SEMANTIC_VERSION,
};

use crate::templates::BuiltInTemplate;
use crate::value_reader::ParameterReader;

pub(crate) struct PickupCollectTemplate;

impl BuiltInTemplate for PickupCollectTemplate {
    fn descriptor(&self) -> TemplateDescriptor {
        TemplateDescriptor {
            id: "toki/pickup_collect".into(),
            display_name: "Pickup Collect".into(),
            category: "inventory".into(),
            description: "Creates overlap-based pickup collection behavior.".into(),
            parameters: vec![
                TemplateParameter {
                    id: "pickup_entity_definition_id".into(),
                    label: "Pickup Entity".into(),
                    description: Some(
                        "Entity definition that acts as the collectible pickup.".into(),
                    ),
                    kind: TemplateParameterKind::EntityDefinitionReference,
                    default: None,
                    required: true,
                },
                TemplateParameter {
                    id: "collector_entity_definition_id".into(),
                    label: "Collector Entity".into(),
                    description: Some("Entity definition that receives the collected item.".into()),
                    kind: TemplateParameterKind::EntityDefinitionReference,
                    default: None,
                    required: true,
                },
                TemplateParameter {
                    id: "item_id".into(),
                    label: "Item Id".into(),
                    description: Some("Inventory item id to add on collection.".into()),
                    kind: TemplateParameterKind::String {
                        multiline: false,
                        min_length: Some(1),
                        max_length: None,
                    },
                    default: None,
                    required: true,
                },
                TemplateParameter {
                    id: "count".into(),
                    label: "Count".into(),
                    description: Some("Number of items granted on collection.".into()),
                    kind: TemplateParameterKind::Integer {
                        min: Some(1),
                        max: Some(9999),
                        step: Some(1),
                    },
                    default: Some(TemplateValue::Integer(1)),
                    required: true,
                },
            ],
        }
    }

    fn instantiate(
        &self,
        parameters: &BTreeMap<String, TemplateValue>,
    ) -> Result<TemplateSemanticPlan, TemplateProviderError> {
        let reader = ParameterReader::new(parameters);
        let pickup_entity_definition_id =
            reader.required_entity_definition_reference("pickup_entity_definition_id")?;
        let collector_entity_definition_id =
            reader.required_entity_definition_reference("collector_entity_definition_id")?;
        let item_id = reader.required_string("item_id")?;
        let count = u32::try_from(reader.required_integer("count")?).map_err(|_| {
            TemplateProviderError::new(
                TemplateProviderErrorCode::SemanticValidation,
                "count must be non-negative",
            )
        })?;

        Ok(TemplateSemanticPlan {
            semantic_version: TEMPLATE_SEMANTIC_VERSION,
            items: vec![TemplateSemanticItem::CreatePickupBehavior {
                id: format!("{pickup_entity_definition_id}_collect"),
                pickup_entity_definition_id,
                collector_entity_definition_id,
                item_id,
                count,
            }],
        })
    }
}
