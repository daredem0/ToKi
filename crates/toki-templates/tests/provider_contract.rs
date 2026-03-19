use std::collections::BTreeMap;

use toki_templates::{
    TemplateContractError, TemplateDescriptor, TemplateEnumOption, TemplateParameter,
    TemplateParameterKind, TemplateValue,
};

fn sample_descriptor() -> TemplateDescriptor {
    TemplateDescriptor {
        id: "toki/test".into(),
        display_name: "Test".into(),
        category: "test".into(),
        description: "Test descriptor".into(),
        parameters: vec![
            TemplateParameter {
                id: "damage".into(),
                label: "Damage".into(),
                description: None,
                kind: TemplateParameterKind::Integer {
                    min: Some(1),
                    max: Some(99),
                    step: Some(1),
                },
                default: Some(TemplateValue::Integer(8)),
                required: true,
            },
            TemplateParameter {
                id: "mode".into(),
                label: "Mode".into(),
                description: None,
                kind: TemplateParameterKind::Enum {
                    options: vec![
                        TemplateEnumOption {
                            id: "melee".into(),
                            label: "Melee".into(),
                            description: None,
                        },
                        TemplateEnumOption {
                            id: "projectile".into(),
                            label: "Projectile".into(),
                            description: None,
                        },
                    ],
                },
                default: Some(TemplateValue::Enum("melee".into())),
                required: true,
            },
            TemplateParameter {
                id: "sound_id".into(),
                label: "Sound".into(),
                description: None,
                kind: TemplateParameterKind::Optional {
                    inner: Box::new(TemplateParameterKind::String {
                        multiline: false,
                        min_length: None,
                        max_length: None,
                    }),
                },
                default: Some(TemplateValue::Optional(None)),
                required: false,
            },
        ],
    }
}

#[test]
fn validate_parameters_accepts_required_and_optional_values() {
    let descriptor = sample_descriptor();
    let mut values = BTreeMap::new();
    values.insert("damage".into(), TemplateValue::Integer(12));
    values.insert("mode".into(), TemplateValue::Enum("projectile".into()));
    values.insert(
        "sound_id".into(),
        TemplateValue::Optional(Some(Box::new(TemplateValue::String("sfx_attack".into())))),
    );

    descriptor
        .validate_parameters(&values)
        .expect("valid input should pass");
}

#[test]
fn validate_parameters_rejects_missing_required_parameter() {
    let descriptor = sample_descriptor();
    let mut values = BTreeMap::new();
    values.insert("mode".into(), TemplateValue::Enum("melee".into()));

    let error = descriptor
        .validate_parameters(&values)
        .expect_err("missing required parameter must fail");

    assert!(matches!(
        error,
        TemplateContractError::MissingRequiredParameter { parameter_id } if parameter_id == "damage"
    ));
}

#[test]
fn validate_parameters_rejects_unexpected_parameter() {
    let descriptor = sample_descriptor();
    let mut values = BTreeMap::new();
    values.insert("damage".into(), TemplateValue::Integer(8));
    values.insert("mode".into(), TemplateValue::Enum("melee".into()));
    values.insert("unknown".into(), TemplateValue::Boolean(true));

    let error = descriptor
        .validate_parameters(&values)
        .expect_err("unexpected parameter must fail");

    assert!(matches!(
        error,
        TemplateContractError::UnexpectedParameter { parameter_id } if parameter_id == "unknown"
    ));
}

#[test]
fn validate_parameters_rejects_invalid_value_type() {
    let descriptor = sample_descriptor();
    let mut values = BTreeMap::new();
    values.insert("damage".into(), TemplateValue::String("bad".into()));
    values.insert("mode".into(), TemplateValue::Enum("melee".into()));

    let error = descriptor
        .validate_parameters(&values)
        .expect_err("invalid value type must fail");

    assert!(matches!(
        error,
        TemplateContractError::ParameterValueTypeMismatch { parameter_id, .. } if parameter_id == "damage"
    ));
}
