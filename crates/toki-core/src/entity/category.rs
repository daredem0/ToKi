use super::model::EntityKind;

pub fn default_category_for_kind(entity_kind: &EntityKind) -> &'static str {
    match entity_kind {
        EntityKind::Player => "human",
        EntityKind::Npc => "creature",
        EntityKind::Item => "item",
        EntityKind::Decoration => "decoration",
        EntityKind::Trigger => "trigger",
        EntityKind::Projectile => "projectile",
    }
}

pub fn runtime_entity_kind_for_category(category: &str) -> EntityKind {
    match category.trim().to_ascii_lowercase().as_str() {
        "item" | "items" => EntityKind::Item,
        "trigger" | "triggers" => EntityKind::Trigger,
        "projectile" | "projectiles" => EntityKind::Projectile,
        "decoration" | "decorations" | "building" | "buildings" | "plant" | "plants" => {
            EntityKind::Decoration
        }
        _ => EntityKind::Npc,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_category_mapping_covers_all_runtime_kinds() {
        assert_eq!(default_category_for_kind(&EntityKind::Player), "human");
        assert_eq!(default_category_for_kind(&EntityKind::Npc), "creature");
        assert_eq!(default_category_for_kind(&EntityKind::Item), "item");
        assert_eq!(
            default_category_for_kind(&EntityKind::Decoration),
            "decoration"
        );
        assert_eq!(default_category_for_kind(&EntityKind::Trigger), "trigger");
        assert_eq!(
            default_category_for_kind(&EntityKind::Projectile),
            "projectile"
        );
    }

    #[test]
    fn category_string_mapping_supports_legacy_aliases() {
        assert_eq!(runtime_entity_kind_for_category("item"), EntityKind::Item);
        assert_eq!(
            runtime_entity_kind_for_category("triggers"),
            EntityKind::Trigger
        );
        assert_eq!(
            runtime_entity_kind_for_category("projectiles"),
            EntityKind::Projectile
        );
        assert_eq!(
            runtime_entity_kind_for_category("building"),
            EntityKind::Decoration
        );
        assert_eq!(
            runtime_entity_kind_for_category("creature"),
            EntityKind::Npc
        );
    }
}
