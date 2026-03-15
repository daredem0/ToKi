use anyhow::Result;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTemplateKind {
    Empty,
    TopDownStarter,
}

impl ProjectTemplateKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Empty => "Empty Project",
            Self::TopDownStarter => "Top-Down Starter",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Empty => "",
            Self::TopDownStarter => {
                "A Game Boy-style starter village with a player, villager, map, and original placeholder art."
            }
        }
    }
}

pub fn populate_project_template(project_path: &Path, template: ProjectTemplateKind) -> Result<()> {
    match template {
        ProjectTemplateKind::Empty => Ok(()),
        ProjectTemplateKind::TopDownStarter => populate_top_down_starter(project_path),
    }
}

fn populate_top_down_starter(project_path: &Path) -> Result<()> {
    for (relative_path, bytes) in top_down_starter_files() {
        let target_path = project_path.join(relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, bytes)?;
    }
    Ok(())
}

fn top_down_starter_files() -> [(&'static str, &'static [u8]); 7] {
    [
        (
            "assets/sprites/terrain.png",
            include_bytes!("../../templates/top_down_starter/assets/sprites/terrain.png"),
        ),
        (
            "assets/sprites/terrain.json",
            include_bytes!("../../templates/top_down_starter/assets/sprites/terrain.json"),
        ),
        (
            "assets/sprites/creatures.png",
            include_bytes!("../../templates/top_down_starter/assets/sprites/creatures.png"),
        ),
        (
            "assets/sprites/creatures.json",
            include_bytes!("../../templates/top_down_starter/assets/sprites/creatures.json"),
        ),
        (
            "assets/tilemaps/starter_overworld.json",
            include_bytes!(
                "../../templates/top_down_starter/assets/tilemaps/starter_overworld.json"
            ),
        ),
        (
            "entities/player.json",
            include_bytes!("../../templates/top_down_starter/entities/player.json"),
        ),
        (
            "entities/villager.json",
            include_bytes!("../../templates/top_down_starter/entities/villager.json"),
        ),
    ]
}

pub fn top_down_main_scene_bytes() -> &'static [u8] {
    include_bytes!("../../templates/top_down_starter/scenes/main.json")
}
