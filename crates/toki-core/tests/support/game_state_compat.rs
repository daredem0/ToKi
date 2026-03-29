use toki_core::assets::{atlas::AtlasMeta, tilemap::TileMap};
use toki_core::entity::{Entity, EntityId, EntityManager, MovementProfile};
use toki_core::game::{
    EntityHealthBar, GameSimulation, GroundShadow, InputAction, InputKey, InputSystem,
    RenderQueryService, RuleSystem, SceneSystem,
};
use toki_core::rules::{Rule, RuleSet};
use toki_core::scene::Scene;
use toki_core::scene_manager::SceneManager;
use toki_core::sprite::SpriteFrame;
use toki_core::sprite_render::SpriteRenderRequest;
use toki_core::game::AudioEvent;
use toki_core::GameState;

#[allow(dead_code)]
pub trait GameStateCompatExt {
    fn update(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> toki_core::GameUpdateResult<AudioEvent>;
    fn update_with_delta(
        &mut self,
        delta_ms: f32,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> toki_core::GameUpdateResult<AudioEvent>;
    fn handle_key_press(&mut self, key: InputKey);
    fn handle_key_release(&mut self, key: InputKey);
    fn handle_profile_key_press(&mut self, profile: MovementProfile, key: InputKey);
    fn handle_profile_key_release(&mut self, profile: MovementProfile, key: InputKey);
    fn handle_profile_action_press(&mut self, profile: MovementProfile, action: InputAction);
    fn handle_profile_action_release(&mut self, profile: MovementProfile, action: InputAction);
    fn set_rules(&mut self, rules: RuleSet);
    fn rules(&self) -> &RuleSet;
    fn rules_mut(&mut self) -> &mut RuleSet;
    fn add_rule(&mut self, rule: Rule);
    fn record_dialog_completion(
        &mut self,
        dialog_id: impl Into<toki_core::DialogId>,
        outcome_id: impl Into<String>,
    );
    fn add_scene(&mut self, scene: Scene);
    fn load_scene(&mut self, scene_name: &str) -> Result<(), String>;
    fn transition_to_scene(&mut self, scene_name: &str, spawn_point_id: &str) -> Result<(), String>;
    fn active_scene(&self) -> Option<&Scene>;
    fn sync_entities_to_active_scene(&mut self);
    fn sync_persistent_entities_to_active_scene(&mut self);
    fn entity_manager(&self) -> &EntityManager;
    fn entity_manager_mut(&mut self) -> &mut EntityManager;
    fn scene_manager(&self) -> &SceneManager;
    fn player_id(&self) -> Option<EntityId>;
    fn player_entity(&self) -> Option<&Entity>;
    fn entities(&self) -> Vec<&Entity>;
    fn entities_owned(&self) -> Vec<Entity>;
    fn player_position(&self) -> glam::IVec2;
    fn current_sprite_frame(&self, atlas: &AtlasMeta, texture_size: glam::UVec2) -> SpriteFrame;
    fn get_entity_sprite_flip_x(&self, entity_id: EntityId) -> bool;
    fn get_sprite_render_requests(&self) -> Vec<SpriteRenderRequest>;
    fn get_renderable_entities(&self) -> Vec<(EntityId, glam::IVec2, glam::UVec2)>;
    fn get_entity_health_bars(&self) -> Vec<EntityHealthBar>;
    fn get_entity_ground_shadows(&self) -> Vec<GroundShadow>;
    fn is_debug_collision_rendering_enabled(&self) -> bool;
    fn get_entity_collision_boxes(&self) -> Vec<(glam::IVec2, glam::UVec2, bool)>;
    fn get_solid_tile_positions(&self, tilemap: &TileMap, atlas: &AtlasMeta) -> Vec<(u32, u32)>;
    fn get_trigger_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)>;
    fn get_rule_velocity(&self, entity_id: EntityId) -> Option<glam::IVec2>;
    fn set_rule_velocity(&mut self, entity_id: EntityId, velocity: glam::IVec2);
}

impl GameStateCompatExt for GameState {
    fn update(
        &mut self,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> toki_core::GameUpdateResult<AudioEvent> {
        GameSimulation::tick_fixed(self, world_bounds, tilemap, atlas)
    }

    fn update_with_delta(
        &mut self,
        delta_ms: f32,
        world_bounds: glam::UVec2,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> toki_core::GameUpdateResult<AudioEvent> {
        GameSimulation::tick_with_delta(self, delta_ms, world_bounds, tilemap, atlas)
    }

    fn handle_key_press(&mut self, key: InputKey) {
        InputSystem::handle_key_press(self.runtime_mut(), key);
    }

    fn handle_key_release(&mut self, key: InputKey) {
        InputSystem::handle_key_release(self.runtime_mut(), key);
    }

    fn handle_profile_key_press(&mut self, profile: MovementProfile, key: InputKey) {
        if matches!(key, InputKey::DebugToggle) {
            InputSystem::handle_key_press(self.runtime_mut(), key);
        } else {
            InputSystem::handle_profile_key_press(self.runtime_mut(), profile, key);
        }
    }

    fn handle_profile_key_release(&mut self, profile: MovementProfile, key: InputKey) {
        InputSystem::handle_profile_key_release(self.runtime_mut(), profile, key);
    }

    fn handle_profile_action_press(&mut self, profile: MovementProfile, action: InputAction) {
        InputSystem::handle_profile_action_press(self.runtime_mut(), profile, action);
    }

    fn handle_profile_action_release(&mut self, profile: MovementProfile, action: InputAction) {
        InputSystem::handle_profile_action_release(self.runtime_mut(), profile, action);
    }

    fn set_rules(&mut self, rules: RuleSet) {
        RuleSystem::set_rules(self, rules);
    }

    fn rules(&self) -> &RuleSet {
        self.scene().active_rules()
    }

    fn rules_mut(&mut self) -> &mut RuleSet {
        self.scene_mut().active_rules_mut()
    }

    fn add_rule(&mut self, rule: Rule) {
        self.scene_mut().active_rules_mut().rules.push(rule);
    }

    fn record_dialog_completion(
        &mut self,
        dialog_id: impl Into<toki_core::DialogId>,
        outcome_id: impl Into<String>,
    ) {
        RuleSystem::record_dialog_completion(self, dialog_id, outcome_id);
    }

    fn add_scene(&mut self, scene: Scene) {
        SceneSystem::add_scene(self, scene);
    }

    fn load_scene(&mut self, scene_name: &str) -> Result<(), String> {
        SceneSystem::load(self, scene_name)
    }

    fn transition_to_scene(&mut self, scene_name: &str, spawn_point_id: &str) -> Result<(), String> {
        SceneSystem::transition(self, scene_name, spawn_point_id)
    }

    fn active_scene(&self) -> Option<&Scene> {
        SceneSystem::active_scene(self)
    }

    fn sync_entities_to_active_scene(&mut self) {
        SceneSystem::sync_entities_to_active_scene(self);
    }

    fn sync_persistent_entities_to_active_scene(&mut self) {
        SceneSystem::sync_persistent_entities_to_active_scene(self);
    }

    fn entity_manager(&self) -> &EntityManager {
        self.world().entity_manager()
    }

    fn entity_manager_mut(&mut self) -> &mut EntityManager {
        self.world_mut().entity_manager_mut()
    }

    fn scene_manager(&self) -> &SceneManager {
        self.scene().scene_manager()
    }

    fn player_id(&self) -> Option<EntityId> {
        self.world().player_id()
    }

    fn player_entity(&self) -> Option<&Entity> {
        self.world()
            .player_id()
            .and_then(|player_id| self.world().entity_manager().get_entity(player_id))
    }

    fn entities(&self) -> Vec<&Entity> {
        self.world()
            .entity_manager()
            .active_entities()
            .iter()
            .filter_map(|&id| self.world().entity_manager().get_entity(id))
            .collect()
    }

    fn entities_owned(&self) -> Vec<Entity> {
        self.world()
            .entity_manager()
            .active_entities()
            .iter()
            .filter_map(|&id| self.world().entity_manager().get_entity(id))
            .cloned()
            .collect()
    }

    fn player_position(&self) -> glam::IVec2 {
        RenderQueryService::new(
            self.world().entity_manager(),
            self.world().player_id(),
            self.runtime().debug_collision_rendering(),
        )
        .player_position()
    }

    fn current_sprite_frame(&self, atlas: &AtlasMeta, texture_size: glam::UVec2) -> SpriteFrame {
        RenderQueryService::new(
            self.world().entity_manager(),
            self.world().player_id(),
            self.runtime().debug_collision_rendering(),
        )
        .current_sprite_frame(atlas, texture_size)
    }

    fn get_entity_sprite_flip_x(&self, entity_id: EntityId) -> bool {
        RenderQueryService::new(
            self.world().entity_manager(),
            self.world().player_id(),
            self.runtime().debug_collision_rendering(),
        )
        .entity_sprite_flip_x(entity_id)
    }

    fn get_sprite_render_requests(&self) -> Vec<SpriteRenderRequest> {
        RenderQueryService::new(
            self.world().entity_manager(),
            self.world().player_id(),
            self.runtime().debug_collision_rendering(),
        )
        .sprite_render_requests()
    }

    fn get_renderable_entities(&self) -> Vec<(EntityId, glam::IVec2, glam::UVec2)> {
        RenderQueryService::new(
            self.world().entity_manager(),
            self.world().player_id(),
            self.runtime().debug_collision_rendering(),
        )
        .renderable_entities()
    }

    fn get_entity_health_bars(&self) -> Vec<EntityHealthBar> {
        RenderQueryService::new(
            self.world().entity_manager(),
            self.world().player_id(),
            self.runtime().debug_collision_rendering(),
        )
        .entity_health_bars()
    }

    fn get_entity_ground_shadows(&self) -> Vec<GroundShadow> {
        RenderQueryService::new(
            self.world().entity_manager(),
            self.world().player_id(),
            self.runtime().debug_collision_rendering(),
        )
        .entity_ground_shadows()
    }

    fn is_debug_collision_rendering_enabled(&self) -> bool {
        self.runtime().debug_collision_rendering()
    }

    fn get_entity_collision_boxes(&self) -> Vec<(glam::IVec2, glam::UVec2, bool)> {
        RenderQueryService::new(
            self.world().entity_manager(),
            self.world().player_id(),
            self.runtime().debug_collision_rendering(),
        )
        .entity_collision_boxes()
    }

    fn get_solid_tile_positions(&self, tilemap: &TileMap, atlas: &AtlasMeta) -> Vec<(u32, u32)> {
        RenderQueryService::new(
            self.world().entity_manager(),
            self.world().player_id(),
            self.runtime().debug_collision_rendering(),
        )
        .solid_tile_positions(tilemap, atlas)
    }

    fn get_trigger_tile_positions(
        &self,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<(u32, u32)> {
        RenderQueryService::new(
            self.world().entity_manager(),
            self.world().player_id(),
            self.runtime().debug_collision_rendering(),
        )
        .trigger_tile_positions(tilemap, atlas)
    }

    fn get_rule_velocity(&self, entity_id: EntityId) -> Option<glam::IVec2> {
        RuleSystem::rule_velocity(self, entity_id)
    }

    fn set_rule_velocity(&mut self, entity_id: EntityId, velocity: glam::IVec2) {
        RuleSystem::set_rule_velocity(self, entity_id, velocity);
    }
}
