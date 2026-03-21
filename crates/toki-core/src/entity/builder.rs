//! Entity builder for fluent entity construction.
//!
//! The builder pattern simplifies creating entities with many optional fields.

use super::types::{
    ControlRole, Entity, EntityAttributes, EntityAudioSettings, EntityId, EntityKind,
};
use crate::collision::CollisionBox;
use glam::{IVec2, UVec2};

/// Builder for constructing Entity instances fluently.
///
/// # Example
/// ```ignore
/// let entity = EntityBuilder::new(1, IVec2::new(100, 200), UVec2::new(32, 32), EntityKind::Npc)
///     .category("creature")
///     .definition_name("goblin")
///     .tags(vec!["enemy".to_string(), "hostile".to_string()])
///     .build();
/// ```
#[derive(Debug)]
pub struct EntityBuilder {
    id: EntityId,
    position: IVec2,
    size: UVec2,
    entity_kind: EntityKind,
    category: Option<String>,
    definition_name: Option<String>,
    control_role: ControlRole,
    audio: EntityAudioSettings,
    attributes: EntityAttributes,
    collision_box: Option<CollisionBox>,
    tags: Vec<String>,
}

impl EntityBuilder {
    /// Create a new entity builder with required fields.
    pub fn new(id: EntityId, position: IVec2, size: UVec2, entity_kind: EntityKind) -> Self {
        Self {
            id,
            position,
            size,
            entity_kind,
            category: None,
            definition_name: None,
            control_role: ControlRole::LegacyDefault,
            audio: EntityAudioSettings::default(),
            attributes: EntityAttributes::default(),
            collision_box: None,
            tags: Vec::new(),
        }
    }

    /// Set the entity category.
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set the source definition name.
    pub fn definition_name(mut self, name: impl Into<String>) -> Self {
        self.definition_name = Some(name.into());
        self
    }

    /// Set the control role.
    pub fn control_role(mut self, role: ControlRole) -> Self {
        self.control_role = role;
        self
    }

    /// Set the audio settings.
    pub fn audio(mut self, audio: EntityAudioSettings) -> Self {
        self.audio = audio;
        self
    }

    /// Set the entity attributes.
    pub fn attributes(mut self, attributes: EntityAttributes) -> Self {
        self.attributes = attributes;
        self
    }

    /// Set the collision box.
    pub fn collision_box(mut self, collision_box: CollisionBox) -> Self {
        self.collision_box = Some(collision_box);
        self
    }

    /// Set the collision box if Some, otherwise leave as None.
    pub fn collision_box_opt(mut self, collision_box: Option<CollisionBox>) -> Self {
        self.collision_box = collision_box;
        self
    }

    /// Set the entity tags.
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add a single tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Build the entity.
    pub fn build(self) -> Entity {
        let category = self.category.unwrap_or_else(|| {
            Self::default_category_for_kind(&self.entity_kind).to_string()
        });

        Entity {
            id: self.id,
            position: self.position,
            size: self.size,
            entity_kind: self.entity_kind,
            category,
            definition_name: self.definition_name,
            control_role: self.control_role,
            audio: self.audio,
            attributes: self.attributes,
            collision_box: self.collision_box,
            tags: self.tags,
            movement_accumulator: glam::Vec2::ZERO,
        }
    }

    fn default_category_for_kind(entity_kind: &EntityKind) -> &'static str {
        match entity_kind {
            EntityKind::Player => "human",
            EntityKind::Npc => "creature",
            EntityKind::Item => "item",
            EntityKind::Decoration => "decoration",
            EntityKind::Trigger => "trigger",
            EntityKind::Projectile => "projectile",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_creates_entity_with_required_fields() {
        let entity = EntityBuilder::new(1, IVec2::new(100, 200), UVec2::new(32, 32), EntityKind::Npc)
            .build();

        assert_eq!(entity.id, 1);
        assert_eq!(entity.position, IVec2::new(100, 200));
        assert_eq!(entity.size, UVec2::new(32, 32));
        assert_eq!(entity.entity_kind, EntityKind::Npc);
        assert_eq!(entity.category, "creature"); // Default for Npc
    }

    #[test]
    fn builder_sets_optional_fields() {
        let entity = EntityBuilder::new(2, IVec2::ZERO, UVec2::ONE, EntityKind::Item)
            .category("treasure")
            .definition_name("gold_coin")
            .tag("valuable")
            .tag("collectible")
            .build();

        assert_eq!(entity.category, "treasure");
        assert_eq!(entity.definition_name, Some("gold_coin".to_string()));
        assert_eq!(entity.tags, vec!["valuable", "collectible"]);
    }

    #[test]
    fn builder_uses_default_category_when_not_specified() {
        let player = EntityBuilder::new(1, IVec2::ZERO, UVec2::ONE, EntityKind::Player).build();
        let item = EntityBuilder::new(2, IVec2::ZERO, UVec2::ONE, EntityKind::Item).build();
        let trigger = EntityBuilder::new(3, IVec2::ZERO, UVec2::ONE, EntityKind::Trigger).build();

        assert_eq!(player.category, "human");
        assert_eq!(item.category, "item");
        assert_eq!(trigger.category, "trigger");
    }

    #[test]
    fn builder_sets_control_role() {
        let entity = EntityBuilder::new(1, IVec2::ZERO, UVec2::ONE, EntityKind::Player)
            .control_role(ControlRole::PlayerCharacter)
            .build();

        assert_eq!(entity.control_role, ControlRole::PlayerCharacter);
    }

    #[test]
    fn builder_sets_collision_box() {
        let collision = CollisionBox::solid_box(UVec2::new(16, 16));
        let entity = EntityBuilder::new(1, IVec2::ZERO, UVec2::ONE, EntityKind::Npc)
            .collision_box(collision.clone())
            .build();

        assert!(entity.collision_box.is_some());
        assert_eq!(entity.collision_box.unwrap().size, collision.size);
    }
}
