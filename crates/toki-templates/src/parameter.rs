use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::TemplateContractError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetReferenceKind {
    Any,
    SpriteAtlas,
    ObjectSheet,
    Tilemap,
    Audio,
    Font,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateEnumOption {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TemplateParameterKind {
    String {
        #[serde(default)]
        multiline: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_length: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_length: Option<u32>,
    },
    Integer {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        step: Option<i64>,
    },
    Float {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        step: Option<f64>,
    },
    Boolean,
    Enum {
        options: Vec<TemplateEnumOption>,
    },
    AssetReference {
        asset_kind: AssetReferenceKind,
    },
    EntityDefinitionReference,
    SceneReference,
    Optional {
        inner: Box<TemplateParameterKind>,
    },
    List {
        item_kind: Box<TemplateParameterKind>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_items: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_items: Option<u32>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TemplateValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Enum(String),
    AssetReference(String),
    EntityDefinitionReference(String),
    SceneReference(String),
    Optional(Option<Box<TemplateValue>>),
    List(Vec<TemplateValue>),
}

impl TemplateValue {
    pub fn kind_name(&self) -> &'static str {
        match self {
            TemplateValue::String(_) => "string",
            TemplateValue::Integer(_) => "integer",
            TemplateValue::Float(_) => "float",
            TemplateValue::Boolean(_) => "boolean",
            TemplateValue::Enum(_) => "enum",
            TemplateValue::AssetReference(_) => "asset_reference",
            TemplateValue::EntityDefinitionReference(_) => "entity_definition_reference",
            TemplateValue::SceneReference(_) => "scene_reference",
            TemplateValue::Optional(_) => "optional",
            TemplateValue::List(_) => "list",
        }
    }
}

impl TemplateParameterKind {
    pub fn expected_kind_name(&self) -> String {
        match self {
            TemplateParameterKind::String { .. } => "string".into(),
            TemplateParameterKind::Integer { .. } => "integer".into(),
            TemplateParameterKind::Float { .. } => "float".into(),
            TemplateParameterKind::Boolean => "boolean".into(),
            TemplateParameterKind::Enum { .. } => "enum".into(),
            TemplateParameterKind::AssetReference { .. } => "asset_reference".into(),
            TemplateParameterKind::EntityDefinitionReference => {
                "entity_definition_reference".into()
            }
            TemplateParameterKind::SceneReference => "scene_reference".into(),
            TemplateParameterKind::Optional { inner } => {
                format!("optional<{}>", inner.expected_kind_name())
            }
            TemplateParameterKind::List { item_kind, .. } => {
                format!("list<{}>", item_kind.expected_kind_name())
            }
        }
    }

    pub fn accepts_value(&self, value: &TemplateValue) -> bool {
        match (self, value) {
            (TemplateParameterKind::String { .. }, TemplateValue::String(_)) => true,
            (TemplateParameterKind::Integer { .. }, TemplateValue::Integer(_)) => true,
            (TemplateParameterKind::Float { .. }, TemplateValue::Float(_)) => true,
            (TemplateParameterKind::Boolean, TemplateValue::Boolean(_)) => true,
            (TemplateParameterKind::Enum { options }, TemplateValue::Enum(selected)) => {
                options.iter().any(|option| option.id == *selected)
            }
            (TemplateParameterKind::AssetReference { .. }, TemplateValue::AssetReference(_)) => {
                true
            }
            (
                TemplateParameterKind::EntityDefinitionReference,
                TemplateValue::EntityDefinitionReference(_),
            ) => true,
            (TemplateParameterKind::SceneReference, TemplateValue::SceneReference(_)) => true,
            (TemplateParameterKind::Optional { inner }, TemplateValue::Optional(value)) => {
                match value {
                    Some(value) => inner.accepts_value(value),
                    None => true,
                }
            }
            (TemplateParameterKind::List { item_kind, .. }, TemplateValue::List(values)) => {
                values.iter().all(|value| item_kind.accepts_value(value))
            }
            _ => false,
        }
    }

    pub fn validate_shape(&self, parameter_id: &str) -> Result<(), TemplateContractError> {
        match self {
            TemplateParameterKind::Enum { options } => {
                if options.is_empty() {
                    return Err(TemplateContractError::EmptyEnumOptions {
                        parameter_id: parameter_id.to_string(),
                    });
                }

                let mut seen = BTreeSet::new();
                for option in options {
                    if !seen.insert(option.id.clone()) {
                        return Err(TemplateContractError::DuplicateEnumOptionId {
                            parameter_id: parameter_id.to_string(),
                            option_id: option.id.clone(),
                        });
                    }
                }
            }
            TemplateParameterKind::Optional { inner } => inner.validate_shape(parameter_id)?,
            TemplateParameterKind::List { item_kind, .. } => {
                item_kind.validate_shape(parameter_id)?
            }
            _ => {}
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateParameter {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub kind: TemplateParameterKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<TemplateValue>,
    #[serde(default)]
    pub required: bool,
}

impl TemplateParameter {
    pub fn validate(&self) -> Result<(), TemplateContractError> {
        if self.id.trim().is_empty() {
            return Err(TemplateContractError::EmptyParameterId);
        }
        if self.label.trim().is_empty() {
            return Err(TemplateContractError::EmptyParameterLabel {
                parameter_id: self.id.clone(),
            });
        }

        self.kind.validate_shape(&self.id)?;

        if let Some(default) = &self.default {
            if !self.kind.accepts_value(default) {
                return Err(TemplateContractError::DefaultValueTypeMismatch {
                    parameter_id: self.id.clone(),
                    expected: self.kind.expected_kind_name(),
                    actual: default.kind_name().to_string(),
                });
            }
        }

        Ok(())
    }
}
