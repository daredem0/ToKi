use chrono::Utc;
use toki_core::project_runtime::{PostProcessMode, ProjectFlagDefinition, QuantizeStrategy};

use super::Project;

#[derive(Debug, Clone)]
pub struct ProjectSettingsDraft {
    pub name: String,
    pub version: String,
    pub description: String,
    pub splash_duration_ms: u64,
    pub scene_persistence: bool,
    pub show_entity_health_bars: bool,
    pub show_ground_shadows: bool,
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub zoom_percent: u32,
    pub vsync: bool,
    pub target_fps: u32,
    pub timing_mode: toki_core::TimingMode,
    pub master_mix_percent: u8,
    pub music_mix_percent: u8,
    pub movement_mix_percent: u8,
    pub collision_mix_percent: u8,
    pub indexed_palette_override: Option<String>,
    pub post_process_mode: PostProcessMode,
    pub post_process_quantize_strategy: QuantizeStrategy,
    pub post_process_tint_color: [u8; 4],
    pub post_process_tint_strength_percent: u8,
    pub post_process_brightness_percent: i16,
    pub post_process_saturation_percent: u8,
    pub post_process_quantize_palette_id: String,
    pub post_process_gb_contrast_percent: i16,
    pub post_process_vignette_strength_percent: u8,
    pub flag_declarations: Vec<ProjectFlagDefinition>,
    pub transition_default_duration_ms: u32,
}

impl ProjectSettingsDraft {
    pub fn from_project(project: &Project) -> Self {
        Self {
            name: project.metadata.project.name.clone(),
            version: project.metadata.project.version.clone(),
            description: project.metadata.project.description.clone(),
            splash_duration_ms: project.metadata.runtime.splash.duration_ms,
            scene_persistence: project.metadata.runtime.scene_persistence,
            show_entity_health_bars: project.metadata.runtime.display.show_entity_health_bars,
            show_ground_shadows: project.metadata.runtime.display.show_ground_shadows,
            resolution_width: project.metadata.runtime.display.resolution_width,
            resolution_height: project.metadata.runtime.display.resolution_height,
            zoom_percent: project.metadata.runtime.display.zoom_percent,
            vsync: project.metadata.runtime.display.vsync,
            target_fps: project.metadata.runtime.display.target_fps,
            timing_mode: project.metadata.runtime.display.timing_mode,
            master_mix_percent: project.metadata.runtime.audio.master_percent,
            music_mix_percent: project.metadata.runtime.audio.music_percent,
            movement_mix_percent: project.metadata.runtime.audio.movement_percent,
            collision_mix_percent: project.metadata.runtime.audio.collision_percent,
            indexed_palette_override: project
                .metadata
                .runtime
                .display
                .indexed_palette_override
                .clone(),
            post_process_mode: project.metadata.runtime.display.post_process.mode,
            post_process_quantize_strategy: project
                .metadata
                .runtime
                .display
                .post_process
                .quantize_strategy,
            post_process_tint_color: project.metadata.runtime.display.post_process.tint_color,
            post_process_tint_strength_percent: project
                .metadata
                .runtime
                .display
                .post_process
                .tint_strength_percent,
            post_process_brightness_percent: project
                .metadata
                .runtime
                .display
                .post_process
                .brightness_percent,
            post_process_saturation_percent: project
                .metadata
                .runtime
                .display
                .post_process
                .saturation_percent,
            post_process_quantize_palette_id: project
                .metadata
                .runtime
                .display
                .post_process
                .quantize_palette_id
                .clone(),
            post_process_gb_contrast_percent: project
                .metadata
                .runtime
                .display
                .post_process
                .gb_contrast_percent,
            post_process_vignette_strength_percent: project
                .metadata
                .runtime
                .display
                .post_process
                .vignette_strength_percent,
            flag_declarations: project.metadata.runtime.flags.declarations.clone(),
            transition_default_duration_ms: project
                .metadata
                .runtime
                .scene_transitions
                .default_duration_ms,
        }
    }
}

pub fn apply_project_settings_draft(project: &mut Project, draft: &ProjectSettingsDraft) -> bool {
    let trimmed_name = draft.name.trim();
    let trimmed_version = draft.version.trim();

    let mut changed = false;
    if !trimmed_name.is_empty() && project.metadata.project.name != trimmed_name {
        project.metadata.project.name = trimmed_name.to_string();
        project.name = trimmed_name.to_string();
        changed = true;
    }
    if !trimmed_version.is_empty() && project.metadata.project.version != trimmed_version {
        project.metadata.project.version = trimmed_version.to_string();
        changed = true;
    }
    if project.metadata.project.description != draft.description {
        project.metadata.project.description = draft.description.clone();
        changed = true;
    }
    if project.metadata.runtime.splash.duration_ms != draft.splash_duration_ms {
        project.metadata.runtime.splash.duration_ms = draft.splash_duration_ms;
        changed = true;
    }
    if project.metadata.runtime.scene_persistence != draft.scene_persistence {
        project.metadata.runtime.scene_persistence = draft.scene_persistence;
        changed = true;
    }
    if project.metadata.runtime.display.show_entity_health_bars != draft.show_entity_health_bars {
        project.metadata.runtime.display.show_entity_health_bars = draft.show_entity_health_bars;
        changed = true;
    }
    if project.metadata.runtime.display.show_ground_shadows != draft.show_ground_shadows {
        project.metadata.runtime.display.show_ground_shadows = draft.show_ground_shadows;
        changed = true;
    }
    if project.metadata.runtime.display.indexed_palette_override != draft.indexed_palette_override {
        project.metadata.runtime.display.indexed_palette_override =
            draft.indexed_palette_override.clone();
        changed = true;
    }
    if project.metadata.runtime.display.post_process.mode != draft.post_process_mode {
        project.metadata.runtime.display.post_process.mode = draft.post_process_mode;
        changed = true;
    }
    if project
        .metadata
        .runtime
        .display
        .post_process
        .quantize_strategy
        != draft.post_process_quantize_strategy
    {
        project
            .metadata
            .runtime
            .display
            .post_process
            .quantize_strategy = draft.post_process_quantize_strategy;
        changed = true;
    }
    if project.metadata.runtime.display.post_process.tint_color != draft.post_process_tint_color {
        project.metadata.runtime.display.post_process.tint_color = draft.post_process_tint_color;
        changed = true;
    }
    if project
        .metadata
        .runtime
        .display
        .post_process
        .tint_strength_percent
        != draft.post_process_tint_strength_percent
    {
        project
            .metadata
            .runtime
            .display
            .post_process
            .tint_strength_percent = draft.post_process_tint_strength_percent;
        changed = true;
    }
    if project
        .metadata
        .runtime
        .display
        .post_process
        .brightness_percent
        != draft.post_process_brightness_percent
    {
        project
            .metadata
            .runtime
            .display
            .post_process
            .brightness_percent = draft.post_process_brightness_percent;
        changed = true;
    }
    if project
        .metadata
        .runtime
        .display
        .post_process
        .saturation_percent
        != draft.post_process_saturation_percent
    {
        project
            .metadata
            .runtime
            .display
            .post_process
            .saturation_percent = draft.post_process_saturation_percent;
        changed = true;
    }
    if project
        .metadata
        .runtime
        .display
        .post_process
        .quantize_palette_id
        != draft.post_process_quantize_palette_id
    {
        project
            .metadata
            .runtime
            .display
            .post_process
            .quantize_palette_id = draft.post_process_quantize_palette_id.clone();
        changed = true;
    }
    if project
        .metadata
        .runtime
        .display
        .post_process
        .gb_contrast_percent
        != draft.post_process_gb_contrast_percent
    {
        project
            .metadata
            .runtime
            .display
            .post_process
            .gb_contrast_percent = draft.post_process_gb_contrast_percent;
        changed = true;
    }
    if project
        .metadata
        .runtime
        .display
        .post_process
        .vignette_strength_percent
        != draft.post_process_vignette_strength_percent
    {
        project
            .metadata
            .runtime
            .display
            .post_process
            .vignette_strength_percent = draft.post_process_vignette_strength_percent;
        changed = true;
    }
    if project.metadata.runtime.flags.declarations != draft.flag_declarations {
        project.metadata.runtime.flags.declarations = draft.flag_declarations.clone();
        changed = true;
    }
    if project.metadata.runtime.scene_transitions.default_duration_ms
        != draft.transition_default_duration_ms
    {
        project.metadata.runtime.scene_transitions.default_duration_ms =
            draft.transition_default_duration_ms.max(1);
        changed = true;
    }
    if project.metadata.runtime.display.resolution_width != draft.resolution_width {
        project.metadata.runtime.display.resolution_width = draft.resolution_width;
        changed = true;
    }
    if project.metadata.runtime.display.resolution_height != draft.resolution_height {
        project.metadata.runtime.display.resolution_height = draft.resolution_height;
        changed = true;
    }
    if project.metadata.runtime.display.zoom_percent != draft.zoom_percent {
        project.metadata.runtime.display.zoom_percent = draft.zoom_percent;
        changed = true;
    }
    if project.metadata.runtime.display.vsync != draft.vsync {
        project.metadata.runtime.display.vsync = draft.vsync;
        changed = true;
    }
    if project.metadata.runtime.display.target_fps != draft.target_fps {
        project.metadata.runtime.display.target_fps = draft.target_fps;
        changed = true;
    }
    if project.metadata.runtime.display.timing_mode != draft.timing_mode {
        project.metadata.runtime.display.timing_mode = draft.timing_mode;
        changed = true;
    }
    if project.audio_config().master_percent != draft.master_mix_percent {
        project.audio_config_mut().master_percent = draft.master_mix_percent;
        changed = true;
    }
    if project.audio_config().music_percent != draft.music_mix_percent {
        project.audio_config_mut().music_percent = draft.music_mix_percent;
        changed = true;
    }
    if project.audio_config().movement_percent != draft.movement_mix_percent {
        project.audio_config_mut().movement_percent = draft.movement_mix_percent;
        changed = true;
    }
    if project.audio_config().collision_percent != draft.collision_mix_percent {
        project.audio_config_mut().collision_percent = draft.collision_mix_percent;
        changed = true;
    }
    if changed {
        project.metadata.project.modified = Utc::now();
        project.is_dirty = true;
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_project_settings_draft_updates_metadata_and_marks_project_dirty() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mut project = Project::new("Demo".to_string(), temp_dir.path().to_path_buf());
        let original_modified = project.metadata.project.modified;

        let draft = ProjectSettingsDraft {
            name: "Renamed Demo".to_string(),
            version: "2.0.0".to_string(),
            description: "Updated description".to_string(),
            splash_duration_ms: 4500,
            scene_persistence: true,
            show_entity_health_bars: true,
            show_ground_shadows: false,
            resolution_width: 320,
            resolution_height: 240,
            zoom_percent: 200,
            vsync: false,
            target_fps: 120,
            timing_mode: toki_core::TimingMode::Delta,
            master_mix_percent: 85,
            music_mix_percent: 70,
            movement_mix_percent: 55,
            collision_mix_percent: 35,
            indexed_palette_override: Some("gb_swamp".to_string()),
            post_process_mode: PostProcessMode::Tint,
            post_process_quantize_strategy: QuantizeStrategy::RgbDistance,
            post_process_tint_color: [12, 34, 56, 255],
            post_process_tint_strength_percent: 65,
            post_process_brightness_percent: 15,
            post_process_saturation_percent: 140,
            post_process_quantize_palette_id: "poison".to_string(),
            post_process_gb_contrast_percent: 12,
            post_process_vignette_strength_percent: 72,
            flag_declarations: vec![ProjectFlagDefinition {
                id: "quest_done".to_string(),
                default_value: toki_core::FlagValue::Bool(false),
            }],
            transition_default_duration_ms: 420,
        };

        let changed = apply_project_settings_draft(&mut project, &draft);

        assert!(changed);
        assert_eq!(project.metadata.project.name, "Renamed Demo");
        assert_eq!(project.metadata.project.version, "2.0.0");
        assert_eq!(project.metadata.project.description, "Updated description");
        assert_eq!(project.metadata.runtime.splash.duration_ms, 4500);
        assert!(project.metadata.runtime.scene_persistence);
        assert!(project.metadata.runtime.display.show_entity_health_bars);
        assert!(!project.metadata.runtime.display.show_ground_shadows);
        assert_eq!(
            project
                .metadata
                .runtime
                .display
                .indexed_palette_override
                .as_deref(),
            Some("gb_swamp")
        );
        assert_eq!(project.metadata.runtime.audio.master_percent, 85);
        assert_eq!(project.metadata.runtime.audio.music_percent, 70);
        assert_eq!(project.metadata.runtime.audio.movement_percent, 55);
        assert_eq!(project.metadata.runtime.audio.collision_percent, 35);
        assert_eq!(
            project.metadata.runtime.display.post_process.mode,
            PostProcessMode::Tint
        );
        assert_eq!(
            project
                .metadata
                .runtime
                .display
                .post_process
                .quantize_strategy,
            QuantizeStrategy::RgbDistance
        );
        assert_eq!(
            project.metadata.runtime.display.post_process.tint_color,
            [12, 34, 56, 255]
        );
        assert_eq!(
            project
                .metadata
                .runtime
                .display
                .post_process
                .tint_strength_percent,
            65
        );
        assert_eq!(
            project
                .metadata
                .runtime
                .display
                .post_process
                .brightness_percent,
            15
        );
        assert_eq!(
            project
                .metadata
                .runtime
                .display
                .post_process
                .saturation_percent,
            140
        );
        assert_eq!(
            project
                .metadata
                .runtime
                .display
                .post_process
                .quantize_palette_id,
            "poison"
        );
        assert_eq!(
            project
                .metadata
                .runtime
                .display
                .post_process
                .gb_contrast_percent,
            12
        );
        assert_eq!(
            project
                .metadata
                .runtime
                .display
                .post_process
                .vignette_strength_percent,
            72
        );
        assert_eq!(project.metadata.runtime.flags.declarations, draft.flag_declarations);
        assert_eq!(
            project.metadata.runtime.scene_transitions.default_duration_ms,
            420
        );
        assert!(project.is_dirty);
        assert!(project.metadata.project.modified >= original_modified);
    }
}
