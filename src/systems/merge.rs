use crate::{
    components::{Mergeable, PhysicsBody},
    resources::MyWorld,
    utils::find_pointed_object::find_pointed_object,
};

use amethyst::{
    core::nalgebra::Vector3,
    core::Transform,
    ecs::{Read, ReadStorage, System, Write, WriteStorage},
    input::InputHandler,
    renderer::Camera,
};
use ncollide3d::world::CollisionObjectHandle;
use nphysics3d::algebra::{Force3, ForceType};
use specs::{Entities, Join};
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
        let w = physics_world.get();
        if input.key_is_down(VirtualKeyCode::E) {
            let x =
                find_pointed_object(
                    &cameras,
                    &transforms,
                    &entities,
                    &physics_world,
                    &physics_bodies,
                    &mergeable,
                )
                .filter(|(_, _, toi)| *toi < MAX_TOI_MERGE)
                .map(|(entity, _, _)| {
                    let center_body = physics_bodies.get(entity).unwrap().0;
                    let mut v = vec![entity];
                    v.extend(w.collider_world().interaction_pairs(false).filter_map(
                        |(c1, c2, _)| {
                            if c1.handle() == center_body || c2.handle() == center_body {
                                let target = if c1.handle() == center_body {
                                    c2.handle()
                                } else {
                                    c1.handle()
                                };
                                (&entities, &physics_bodies, &mergeable)
                                    .join()
                                    .find(|(_, pb, _)| pb.0 == target)
                                    .map(|(e, _, _)| e)
                            } else {
                                None
                            }
                        },
                    ));
                    v
                });
            println!("{:?}", x)
        } else {
        }
    }
}
