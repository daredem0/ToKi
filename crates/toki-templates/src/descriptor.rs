use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{TemplateContractError, TemplateParameter};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateDescriptor {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub description: String,
    #[serde(default)]
    pub parameters: Vec<TemplateParameter>,
}

impl TemplateDescriptor {
    pub fn validate(&self) -> Result<(), TemplateContractError> {
        if self.id.trim().is_empty() {
            return Err(TemplateContractError::EmptyTemplateId);
        }
        if self.display_name.trim().is_empty() {
            return Err(TemplateContractError::EmptyTemplateDisplayName);
        }

        let mut seen = BTreeSet::new();
        for parameter in &self.parameters {
            parameter.validate()?;
            if !seen.insert(parameter.id.clone()) {
                return Err(TemplateContractError::DuplicateParameterId {
                    id: parameter.id.clone(),
                });
            }
        }

        Ok(())
    }

    pub fn validate_parameters(
        &self,
        parameters: &BTreeMap<String, crate::TemplateValue>,
    ) -> Result<(), TemplateContractError> {
        self.validate()?;

        let parameter_defs: BTreeMap<_, _> = self.parameters.iter().map(|p| (&p.id, p)).collect();

        for (parameter_id, value) in parameters {
            let parameter = parameter_defs.get(parameter_id).ok_or_else(|| {
                TemplateContractError::UnexpectedParameter {
                    parameter_id: parameter_id.clone(),
                }
            })?;

            if !parameter.kind.accepts_value(value) {
                return Err(TemplateContractError::ParameterValueTypeMismatch {
                    parameter_id: parameter_id.clone(),
                    expected: parameter.kind.expected_kind_name(),
                    actual: value.kind_name().to_string(),
                });
            }
        }

        for parameter in &self.parameters {
            if parameter.required && !parameters.contains_key(&parameter.id) {
                return Err(TemplateContractError::MissingRequiredParameter {
                    parameter_id: parameter.id.clone(),
                });
            }
        }

        Ok(())
    }

    pub fn resolve_parameters(
        &self,
        parameters: &BTreeMap<String, crate::TemplateValue>,
    ) -> Result<BTreeMap<String, crate::TemplateValue>, TemplateContractError> {
        self.validate_parameters(parameters)?;

        let mut resolved = parameters.clone();
        for parameter in &self.parameters {
            if !resolved.contains_key(&parameter.id) {
                if let Some(default) = &parameter.default {
                    resolved.insert(parameter.id.clone(), default.clone());
                }
            }
        }

        Ok(resolved)
    }
}
