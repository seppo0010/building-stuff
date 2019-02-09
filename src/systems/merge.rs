use crate::{
    components::{Mergeable, PhysicsBody},
    resources::MyWorld,
    utils::find_pointed_object::find_pointed_object,
};

use amethyst::{
    core::Transform,
    ecs::{Read, ReadStorage, System, Write, WriteStorage},
    input::InputHandler,
    renderer::Camera,
};
use specs::Entities;
use winit::VirtualKeyCode;

const MAX_TOI_MERGE: f32 = 5.0;

#[derive(Default)]
pub struct MergeSystem;

impl MergeSystem {}

impl<'s> System<'s> for MergeSystem {
    type SystemData = (
        Entities<'s>,
        Write<'s, MyWorld>,
        WriteStorage<'s, PhysicsBody>,
        Read<'s, InputHandler<String, String>>,
        ReadStorage<'s, Camera>,
        ReadStorage<'s, Transform>,
        ReadStorage<'s, Mergeable>,
    );
    fn run(
        &mut self,
        (entities, mut physics_world, mut physics_bodies, input, cameras, transforms, mergeable): Self::SystemData,
    ) {
        if input.key_is_down(VirtualKeyCode::E) {
            find_pointed_object(
                &cameras,
                &transforms,
                &entities,
                &physics_world,
                &physics_bodies,
                &mergeable,
            )
            .filter(|(_, _, toi)| *toi < MAX_TOI_MERGE);
        } else {
        }
    }
}
