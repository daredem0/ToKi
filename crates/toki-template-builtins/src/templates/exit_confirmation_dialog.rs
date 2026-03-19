use std::collections::BTreeMap;

use toki_templates::{
    TemplateDescriptor, TemplateParameter, TemplateParameterKind, TemplateProviderError,
    TemplateSemanticItem, TemplateSemanticPlan, TemplateSurfaceAction, TemplateValue,
    TEMPLATE_SEMANTIC_VERSION,
};

use crate::templates::BuiltInTemplate;
use crate::value_reader::ParameterReader;

pub(crate) struct ExitConfirmationDialogTemplate;

impl BuiltInTemplate for ExitConfirmationDialogTemplate {
    fn descriptor(&self) -> TemplateDescriptor {
        TemplateDescriptor {
            id: "toki/exit_confirmation_dialog".into(),
            display_name: "Exit Confirmation Dialog".into(),
            category: "ui".into(),
            description: "Creates a confirmation dialog that exits the runtime when confirmed."
                .into(),
            parameters: vec![
                TemplateParameter {
                    id: "dialog_id".into(),
                    label: "Dialog Id".into(),
                    description: Some("Stable authored id for the dialog.".into()),
                    kind: TemplateParameterKind::String {
                        multiline: false,
                        min_length: Some(1),
                        max_length: None,
                    },
                    default: Some(TemplateValue::String("exit_confirm".into())),
                    required: true,
                },
                TemplateParameter {
                    id: "title".into(),
                    label: "Title".into(),
                    description: Some("Dialog title text.".into()),
                    kind: TemplateParameterKind::String {
                        multiline: false,
                        min_length: Some(1),
                        max_length: None,
                    },
                    default: Some(TemplateValue::String("Exit Game?".into())),
                    required: true,
                },
                TemplateParameter {
                    id: "body".into(),
                    label: "Body".into(),
                    description: Some("Dialog body text.".into()),
                    kind: TemplateParameterKind::String {
                        multiline: true,
                        min_length: Some(1),
                        max_length: None,
                    },
                    default: Some(TemplateValue::String(
                        "Unsaved progress may be lost.".into(),
                    )),
                    required: true,
                },
                TemplateParameter {
                    id: "confirm_label".into(),
                    label: "Confirm Label".into(),
                    description: Some("Text shown on the confirm button.".into()),
                    kind: TemplateParameterKind::String {
                        multiline: false,
                        min_length: Some(1),
                        max_length: None,
                    },
                    default: Some(TemplateValue::String("Exit".into())),
                    required: true,
                },
                TemplateParameter {
                    id: "cancel_label".into(),
                    label: "Cancel Label".into(),
                    description: Some("Text shown on the cancel button.".into()),
                    kind: TemplateParameterKind::String {
                        multiline: false,
                        min_length: Some(1),
                        max_length: None,
                    },
                    default: Some(TemplateValue::String("Cancel".into())),
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
        let dialog_id = reader.required_string("dialog_id")?;
        let title = reader.required_string("title")?;
        let body = reader.required_string("body")?;
        let confirm_label = reader.required_string("confirm_label")?;
        let cancel_label = reader.required_string("cancel_label")?;

        Ok(TemplateSemanticPlan {
            semantic_version: TEMPLATE_SEMANTIC_VERSION,
            items: vec![TemplateSemanticItem::CreateConfirmationDialog {
                id: dialog_id,
                title,
                body,
                confirm_label,
                cancel_label,
                confirm_action: TemplateSurfaceAction::ExitRuntime,
                cancel_action: TemplateSurfaceAction::CloseSurface,
            }],
        })
    }
}
