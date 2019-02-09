use std::{cmp::Ordering, f32};

use crate::{components::PhysicsBody, resources::MyWorld};

use amethyst::{
    core::{
        nalgebra::{Point3, Vector3},
        Transform,
    },
    ecs::{Join, ReadStorage, Write, WriteStorage},
    renderer::Camera,
};

use ncollide3d::query::{Ray, RayCast};
use specs::{Component, Entities, Entity};

pub fn find_pointed_object<'a, T: Component>(
    cameras: &ReadStorage<Camera>,
    transforms: &ReadStorage<Transform>,
    entities: &Entities,
    physics_world: &Write<MyWorld>,
    physics_bodies: &WriteStorage<PhysicsBody>,
    filter_type: &'a ReadStorage<T>,
) -> Option<(Entity, &'a T, f32)> {
    let isometry = (cameras, transforms).join().next().unwrap().1.isometry();
    let ray = Ray::new(
        Point3::new(
            isometry.translation.vector.x,
            isometry.translation.vector.y,
            isometry.translation.vector.z,
        ),
        -(isometry.rotation * Vector3::z()),
    );

    let world = physics_world.get();
    (entities, physics_bodies, filter_type)
        .join()
        .flat_map(|(e, b, g)| {
            let co = world
                .collider_world()
                .as_collider_world()
                .collision_object(b.0)
                .unwrap();
            co.shape()
                .toi_with_ray(co.position(), &ray, true)
                .map(|x| (e, b, x, g))
        })
        .min_by(|(_, _, toi1, _), (_, _, toi2, _)| {
            toi1.partial_cmp(&toi2).unwrap_or(Ordering::Equal)
        })
        .map(|(e, _, toi, g)| (e, g, toi))
}
