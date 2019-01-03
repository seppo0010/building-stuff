use crate::{components::PhysicsBody, resources::MyWorld};

use amethyst::{
    core::Transform,
    ecs::{Join, ReadStorage, System, Write, WriteStorage},
};

#[derive(Default)]
pub struct PhysicsSystem;

impl<'s> System<'s> for PhysicsSystem {
    type SystemData = (
        Write<'s, MyWorld>,
        WriteStorage<'s, Transform>,
        ReadStorage<'s, PhysicsBody>,
    );
    fn run(&mut self, (mut physics_world, mut transforms, bodies): Self::SystemData) {
        physics_world.step();
        for (mut t, body) in (&mut transforms, &bodies).join() {
            if let Some(pos) = physics_world
                .collider_body_handle(body.0)
                .and_then(|bh| physics_world.rigid_body(bh))
                .map(|co| co.position())
            {
                *t.translation_mut() = pos.translation.vector;
                *t.rotation_mut() = pos.rotation;
            }
        }
    }
}