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
}
