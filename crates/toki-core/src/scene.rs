use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::animation::AnimationState;
use crate::assets::atlas::AtlasMeta;
use crate::assets::tilemap::TileMap;
use crate::entity::{Entity, EntityId, EntityManager};
use crate::events::{GameEvent, GameUpdateResult};
use crate::sprite::{SpriteFrame, SpriteInstance};

#[derive(Debug, Serialize, Deserialize)]
pub struct Scene {}
