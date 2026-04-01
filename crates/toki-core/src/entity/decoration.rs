use super::{
    Entity, EntityBuilder, EntityGrounding, EntityId, EntityKind, EntityRendering,
    StaticObjectRenderDef,
};
use crate::collision::CollisionBox;
use glam::{IVec2, UVec2};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecorationSpec {
    pub position: IVec2,
    pub size: UVec2,
    pub sheet: String,
    pub object_name: String,
    pub grounding: EntityGrounding,
    pub visible: bool,
    pub solid: bool,
}

impl DecorationSpec {
    pub fn new(
        position: IVec2,
        size: UVec2,
        sheet: impl Into<String>,
        object_name: impl Into<String>,
    ) -> Self {
        Self {
            position,
            size,
            sheet: sheet.into(),
            object_name: object_name.into(),
            grounding: EntityGrounding::default(),
            visible: true,
            solid: true,
        }
    }
}

pub fn build_decoration_entity(id: EntityId, spec: DecorationSpec) -> Entity {
    let collision_box = decoration_collision_box(spec.size, &spec.grounding, spec.solid);

    EntityBuilder::new(id, spec.position, spec.size, EntityKind::Decoration)
        .rendering(EntityRendering {
            visible: spec.visible,
            static_object_render: Some(StaticObjectRenderDef {
                sheet: spec.sheet,
                object_name: spec.object_name,
            }),
            grounding: spec.grounding,
            ..EntityRendering::default()
        })
        .solid(spec.solid)
        .active(false)
        .collision_box_opt(collision_box)
        .build()
}

pub fn decoration_collision_box(
    render_size: UVec2,
    grounding: &EntityGrounding,
    solid: bool,
) -> Option<CollisionBox> {
    if !solid {
        return None;
    }

    let footprint = grounding.resolved_footprint(render_size, None);
    Some(CollisionBox::new(
        IVec2::new(footprint.offset[0], footprint.offset[1]),
        UVec2::new(footprint.size[0], footprint.size[1]),
        false,
    ))
}
