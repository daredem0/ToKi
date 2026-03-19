use std::collections::BTreeMap;

use toki_templates::{TemplateProviderError, TemplateProviderErrorCode, TemplateValue};

pub(crate) struct ParameterReader<'a> {
    values: &'a BTreeMap<String, TemplateValue>,
}

impl<'a> ParameterReader<'a> {
    pub(crate) fn new(values: &'a BTreeMap<String, TemplateValue>) -> Self {
        Self { values }
    }

    pub(crate) fn required_string(&self, id: &str) -> Result<String, TemplateProviderError> {
        match self.values.get(id) {
            Some(TemplateValue::String(value)) => Ok(value.clone()),
            other => Err(type_error(id, "string", other)),
        }
    }

    pub(crate) fn required_integer(&self, id: &str) -> Result<i64, TemplateProviderError> {
        match self.values.get(id) {
            Some(TemplateValue::Integer(value)) => Ok(*value),
            other => Err(type_error(id, "integer", other)),
        }
    }

    pub(crate) fn required_enum(&self, id: &str) -> Result<String, TemplateProviderError> {
        match self.values.get(id) {
            Some(TemplateValue::Enum(value)) => Ok(value.clone()),
            other => Err(type_error(id, "enum", other)),
        }
    }

    pub(crate) fn required_entity_definition_reference(
        &self,
        id: &str,
    ) -> Result<String, TemplateProviderError> {
        match self.values.get(id) {
            Some(TemplateValue::EntityDefinitionReference(value)) => Ok(value.clone()),
            other => Err(type_error(id, "entity_definition_reference", other)),
        }
    }

    pub(crate) fn optional_string(
        &self,
        id: &str,
    ) -> Result<Option<String>, TemplateProviderError> {
        match self.values.get(id) {
            None => Ok(None),
            Some(TemplateValue::Optional(None)) => Ok(None),
            Some(TemplateValue::Optional(Some(value))) => match value.as_ref() {
                TemplateValue::String(value) => Ok(Some(value.clone())),
                other => Err(type_error(id, "optional<string>", Some(other))),
            },
            other => Err(type_error(id, "optional<string>", other)),
        }
    }

    pub(crate) fn optional_entity_definition_reference(
        &self,
        id: &str,
    ) -> Result<Option<String>, TemplateProviderError> {
        match self.values.get(id) {
            None => Ok(None),
            Some(TemplateValue::Optional(None)) => Ok(None),
            Some(TemplateValue::Optional(Some(value))) => match value.as_ref() {
                TemplateValue::EntityDefinitionReference(value) => Ok(Some(value.clone())),
                other => Err(type_error(
                    id,
                    "optional<entity_definition_reference>",
                    Some(other),
                )),
            },
            other => Err(type_error(
                id,
                "optional<entity_definition_reference>",
                other,
            )),
        }
    }

    pub(crate) fn optional_asset_reference(
        &self,
        id: &str,
    ) -> Result<Option<String>, TemplateProviderError> {
        match self.values.get(id) {
            None => Ok(None),
            Some(TemplateValue::Optional(None)) => Ok(None),
            Some(TemplateValue::Optional(Some(value))) => match value.as_ref() {
                TemplateValue::AssetReference(value) => Ok(Some(value.clone())),
                other => Err(type_error(id, "optional<asset_reference>", Some(other))),
            },
            other => Err(type_error(id, "optional<asset_reference>", other)),
        }
    }
}

fn type_error(id: &str, expected: &str, actual: Option<&TemplateValue>) -> TemplateProviderError {
    let actual = actual.map_or("missing", TemplateValue::kind_name);
    TemplateProviderError::new(
        TemplateProviderErrorCode::Internal,
        format!("parameter '{id}' had unexpected resolved type: expected {expected}, got {actual}"),
    )
}
