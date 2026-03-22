use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::game::GameState;
use crate::rules::RuleTrigger;

use super::RuleCommand;

impl GameState {
    /// Collect reactive rule commands based on frame events and post-movement world state.
    pub(in crate::game) fn collect_reactive_rule_commands(
        &mut self,
        player_moved: bool,
        tilemap: &TileMap,
        atlas: &AtlasMeta,
    ) -> Vec<RuleCommand> {
        let mut reactive_rule_commands = Vec::new();

        if player_moved {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnPlayerMove,
                &mut reactive_rule_commands,
            );
        }

        let collision_events = std::mem::take(&mut self.rule_runtime.frame_collisions);
        for event in collision_events {
            self.collect_rule_commands_for_collision(&event, &mut reactive_rule_commands);
        }

        let damage_events = std::mem::take(&mut self.rule_runtime.frame_damage_events);
        for event in damage_events {
            self.collect_rule_commands_for_damage(&event, &mut reactive_rule_commands);
        }

        let death_events = std::mem::take(&mut self.rule_runtime.frame_death_events);
        for event in death_events {
            self.collect_rule_commands_for_death(&event, &mut reactive_rule_commands);
        }

        let interaction_events = std::mem::take(&mut self.rule_runtime.frame_interactions);
        for event in &interaction_events {
            self.collect_rule_commands_for_interaction(event, &mut reactive_rule_commands);
        }

        self.collect_rule_commands_for_tile_transitions(tilemap, &mut reactive_rule_commands);

        if self.any_entity_overlaps_trigger_tile(tilemap, atlas) {
            self.collect_rule_commands_for_trigger(
                RuleTrigger::OnTrigger,
                &mut reactive_rule_commands,
            );
        }

        reactive_rule_commands
    }
}

#[cfg(test)]
mod tests {
    use crate::assets::atlas::AtlasMeta;
    use crate::assets::tilemap::TileMap;
    use crate::game::{AudioChannel, DamageEvent, GameState};
    use crate::rules::{Rule, RuleAction, RuleCondition, RuleSet, RuleSoundChannel, RuleTrigger};

    use super::RuleCommand;

    #[test]
    fn reactive_rule_pipeline_collects_commands_from_frame_events_and_player_motion() {
        let mut state = GameState::new_empty();
        let self_id = state.spawn_player_like_npc(glam::IVec2::new(0, 0));
        let other_id = state.spawn_player_like_npc(glam::IVec2::new(16, 0));

        state.set_rules(RuleSet {
            rules: vec![
                Rule {
                    id: "on_move".to_string(),
                    trigger: RuleTrigger::OnPlayerMove,
                    conditions: vec![RuleCondition::Always],
                    actions: vec![RuleAction::PlaySound {
                        channel: RuleSoundChannel::Movement,
                        sound_id: "player_move".to_string(),
                    }],
                    priority: 0,
                    enabled: true,
                    once: false,
                },
                Rule {
                    id: "on_damage".to_string(),
                    trigger: RuleTrigger::OnDamaged { entity: None },
                    conditions: vec![RuleCondition::Always],
                    actions: vec![RuleAction::PlaySound {
                        channel: RuleSoundChannel::Collision,
                        sound_id: "damage_taken".to_string(),
                    }],
                    priority: 0,
                    enabled: true,
                    once: false,
                },
            ],
        });

        state.rule_runtime.frame_damage_events.push(DamageEvent {
            victim: self_id,
            attacker: Some(other_id),
        });
        let commands = state.collect_reactive_rule_commands(
            true,
            &TileMap {
                size: glam::UVec2::new(1, 1),
                tile_size: glam::UVec2::new(16, 16),
                atlas: std::path::PathBuf::new(),
                tiles: vec!["default".to_string()],
                objects: vec![],
            },
            &AtlasMeta {
                image: std::path::PathBuf::new(),
                tile_size: glam::UVec2::new(16, 16),
                tiles: std::collections::HashMap::new(),
            },
        );

        assert!(commands.iter().any(|command| matches!(
            command,
            RuleCommand::PlaySound { channel: AudioChannel::Movement, sound_id }
                if sound_id == "player_move"
        )));
        assert!(commands.iter().any(|command| matches!(
            command,
            RuleCommand::PlaySound { channel: AudioChannel::Collision, sound_id }
                if sound_id == "damage_taken"
        )));
    }
}
