use jsonschema::JSONSchema;
use serde_json::Value;

pub fn compile_schema(schema_str: &str, label: &str) -> JSONSchema {
    let schema: Value =
        serde_json::from_str(schema_str).unwrap_or_else(|_| panic!("{label} schema should parse"));
    JSONSchema::compile(&schema).unwrap_or_else(|_| panic!("{label} schema should compile"))
}

pub fn assert_valid(schema: &JSONSchema, doc: &Value) {
    if let Err(errors) = schema.validate(doc) {
        let details = errors.map(|error| error.to_string()).collect::<Vec<_>>();
        panic!(
            "expected schema-valid document, got: {}",
            details.join(" | ")
        );
    }
}

pub fn assert_invalid(schema: &JSONSchema, doc: &Value) {
    assert!(
        schema.validate(doc).is_err(),
        "expected schema-invalid document"
    );
}
