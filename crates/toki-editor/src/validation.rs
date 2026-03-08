use anyhow::Result;
use jsonschema::JSONSchema;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::project::ProjectAssets;

pub struct AssetValidator {
    schemas: HashMap<String, JSONSchema>,
}

impl AssetValidator {
    pub fn new() -> Result<Self> {
        let mut schemas = HashMap::new();

        let schema_files = [
            ("scene", include_str!("../../../schemas/scene.json")),
            ("entity", include_str!("../../../schemas/entity.json")),
            ("atlas", include_str!("../../../schemas/atlas.json")),
            ("map", include_str!("../../../schemas/map.json")),
        ];

        for (schema_type, schema_content) in schema_files {
            let schema_value: Value = serde_json::from_str(schema_content)
                .map_err(|e| anyhow::anyhow!("Failed to parse {} schema: {}", schema_type, e))?;

            let compiled_schema = JSONSchema::compile(&schema_value)
                .map_err(|e| anyhow::anyhow!("Failed to compile {} schema: {}", schema_type, e))?;

            schemas.insert(schema_type.to_string(), compiled_schema);
        }

        tracing::info!("Loaded {} JSON schemas for validation", schemas.len());
        Ok(Self { schemas })
    }

    pub fn validate_project_assets(&self, project_assets: &ProjectAssets) -> Result<()> {
        tracing::info!("🔍 Starting project asset validation");

        let mut total_files = 0;
        let mut valid_files = 0;

        // Validate scenes
        for (scene_name, scene_asset) in &project_assets.scenes {
            total_files += 1;
            if self.validate_and_log_file(&scene_asset.path, "scene", scene_name) {
                valid_files += 1;
            }
        }

        // Validate entities
        for (entity_name, entity_asset) in &project_assets.entities {
            total_files += 1;
            if self.validate_and_log_file(&entity_asset.path, "entity", entity_name) {
                valid_files += 1;
            }
        }

        // Validate sprite atlases
        for (atlas_name, atlas_asset) in &project_assets.sprite_atlases {
            total_files += 1;
            if self.validate_and_log_file(&atlas_asset.path, "atlas", atlas_name) {
                valid_files += 1;
            }
        }

        // Validate tilemaps
        for (map_name, map_asset) in &project_assets.tilemaps {
            total_files += 1;
            if self.validate_and_log_file(&map_asset.path, "map", map_name) {
                valid_files += 1;
            }
        }

        tracing::info!(
            "✅ Project validation complete: {}/{} files valid",
            valid_files,
            total_files
        );

        Ok(())
    }

    fn validate_and_log_file(&self, file_path: &Path, schema_type: &str, asset_name: &str) -> bool {
        let file_path_str = file_path.to_string_lossy();

        // Read and parse JSON
        let content = match fs::read_to_string(file_path) {
            Ok(content) => content,
            Err(e) => {
                tracing::error!(
                    "❌ {} '{}': Failed to read file {}: {}",
                    schema_type,
                    asset_name,
                    file_path_str,
                    e
                );
                return false;
            }
        };

        let json_value: Value = match serde_json::from_str(&content) {
            Ok(value) => value,
            Err(e) => {
                tracing::error!(
                    "❌ {} '{}': Invalid JSON in {}: {}",
                    schema_type,
                    asset_name,
                    file_path_str,
                    e
                );
                return false;
            }
        };

        // Get schema
        let schema = match self.schemas.get(schema_type) {
            Some(schema) => schema,
            None => {
                tracing::error!(
                    "❌ {} '{}': No schema found for type: {}",
                    schema_type,
                    asset_name,
                    schema_type
                );
                return false;
            }
        };

        // Validate
        let validation_result = schema.validate(&json_value);
        match validation_result {
            Ok(_) => {
                tracing::info!("✅ {} '{}': Valid", schema_type, asset_name);
                true
            }
            Err(errors) => {
                tracing::error!(
                    "❌ {} '{}': Schema validation failed:",
                    schema_type,
                    asset_name
                );
                for error in errors {
                    tracing::error!("   At '{}': {}", error.instance_path, error);
                }
                false
            }
        }
    }
}
