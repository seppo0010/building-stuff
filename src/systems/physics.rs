use crate::{components::PhysicsBody, resources::MyWorld};

use amethyst::{
    core::{timing::Time, Transform},
    ecs::{Join, Read, ReadStorage, System, Write, WriteStorage},
};

const MAX_STEPS_PER_RUN: u8 = 4;

#[derive(Default)]
pub struct PhysicsSystem {
    time_accumulator: f32,
}

impl<'s> System<'s> for PhysicsSystem {
    type SystemData = (
        Write<'s, MyWorld>,
        WriteStorage<'s, Transform>,
        ReadStorage<'s, PhysicsBody>,
        Read<'s, Time>,
    );
    fn run(&mut self, (mut physics_world, mut transforms, bodies, time): Self::SystemData) {
        self.time_accumulator += time.delta_seconds();
        let timestep = physics_world.timestep();
        for _ in 0..MAX_STEPS_PER_RUN {
            if self.time_accumulator < timestep {
                break;
            }
            physics_world.step();
            self.time_accumulator -= timestep;
        }
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
