use std::{cmp::Ordering, f32};

use crate::{
    components::{Grabbable, PhysicsBody},
    resources::MyWorld,
};

use amethyst::{
    core::{nalgebra::Vector3, timing::Time, Transform},
    ecs::{Join, Read, ReadStorage, System, Write, WriteStorage},
    input::InputHandler,
    renderer::{Camera, MouseButton},
    shrev::{EventChannel, ReaderId},
};

use na::{Isometry3, Point3, UnitQuaternion, Vector3 as PhysicsVector3};

use ncollide3d::query::{Ray, RayCast};
use nphysics3d::{
    force_generator::{ConstantAcceleration, ForceGeneratorHandle},
    object::RigidBody,
};
use specs::{prelude::Resources, Entities, Entity};
use winit::{DeviceEvent, Event};

const MAGIC_ANGULAR_VELOCITY_MULTIPLIER: f32 = 50.0;
const MAX_TOI_GRAB: f32 = 4.0;

struct SelectedObject {
    entity: Entity,
    previous_camera_position: Isometry3<f32>,
    force: ForceGeneratorHandle,
    distance: f32,
    box_forward: Vector3<f32>,
    box_up: Vector3<f32>,
}

#[derive(Default)]
pub struct PointingSystem {
    selected_object: Option<SelectedObject>,
    did_release_click: bool,
    event_reader: Option<ReaderId<Event>>,
}

impl PointingSystem {
    fn find_current_ray(
        &self,
        cameras: &ReadStorage<Camera>,
        transforms: &ReadStorage<Transform>,
    ) -> (Ray<f32>, Isometry3<f32>) {
        let isometry = (cameras, transforms).join().next().unwrap().1.isometry();
        let r = isometry.rotation * Vector3::z();
        (
            Ray::new(
                Point3::new(
                    isometry.translation.vector.x,
                    isometry.translation.vector.y,
                    isometry.translation.vector.z,
                ),
                PhysicsVector3::new(-r.x, -r.y, -r.z),
            ),
            *isometry,
        )
    }

    fn find_pointed_object(
        &self,
        ray: &Ray<f32>,
        entities: &Entities,
        physics_world: &Write<MyWorld>,
        physics_bodies: &WriteStorage<PhysicsBody>,
        grabbables: &ReadStorage<Grabbable>,
    ) -> Option<(Entity, f32)> {
        (entities, physics_bodies, grabbables)
            .join()
            .flat_map(|(e, b, _)| {
                let co = physics_world
                    .collision_world()
                    .collision_object(b.0)
                    .unwrap();
                co.shape()
                    .toi_with_ray(co.position(), &ray, true)
                    .map(|x| (e, b, x))
            })
            .min_by(|(_, _, toi1), (_, _, toi2)| toi1.partial_cmp(&toi2).unwrap_or(Ordering::Equal))
            .map(|(e, _, toi)| (e, toi))
    }

    fn get_selected_object_rigid_body_mut<'a>(
        &mut self,
        physics_bodies: &WriteStorage<PhysicsBody>,
        world: &'a mut Write<MyWorld>,
    ) -> Option<&'a mut RigidBody<f32>> {
        self.selected_object
            .as_mut()
            .and_then(|so| physics_bodies.get(so.entity))
            .and_then(|body| world.collider_body_handle(body.0))
            .and_then(move |bh| world.rigid_body_mut(bh))
    }

    fn move_selected_object(
        &mut self,
        cameras: &ReadStorage<Camera>,
        transforms: &ReadStorage<Transform>,
        physics_bodies: &WriteStorage<PhysicsBody>,
        world: &mut Write<MyWorld>,
        time: &Read<Time>,
    ) {
        let camera_isometry = self.find_current_ray(cameras, transforms).1;
        let rb = match self.get_selected_object_rigid_body_mut(physics_bodies, world) {
            Some(x) => x,
            None => return,
        };
        let so = self.selected_object.as_mut().unwrap();
        let linear = camera_isometry.translation.vector
            - so.previous_camera_position.translation.vector
            + (so.previous_camera_position.rotation * Vector3::z()
                - camera_isometry.rotation * Vector3::z())
                * so.distance;
        let angular = (rb.position().rotation * so.box_forward)
            .cross(&(camera_isometry.rotation * Vector3::z()))
            + (rb.position().rotation * so.box_up)
                .cross(&(camera_isometry.rotation * Vector3::y()));
        rb.set_linear_velocity(linear / time.delta_seconds());
        rb.set_angular_velocity(angular * MAGIC_ANGULAR_VELOCITY_MULTIPLIER);
        so.previous_camera_position = camera_isometry;
    }

    fn grab_object(
        &mut self,
        entities: &Entities,
        cameras: &ReadStorage<Camera>,
        physics_world: &mut Write<MyWorld>,
        transforms: &ReadStorage<Transform>,
        physics_bodies: &WriteStorage<PhysicsBody>,
        grabbables: &ReadStorage<Grabbable>,
    ) {
        let (ray, camera_isometry) = self.find_current_ray(&cameras, &transforms);

        self.selected_object = self
            .find_pointed_object(&ray, entities, &physics_world, physics_bodies, grabbables)
            .filter(|(_, toi)| *toi < MAX_TOI_GRAB)
            .map(|(entity, toi)| {
                let mut f = ConstantAcceleration::new(
                    -physics_world.gravity(),
                    Vector3::new(0.0, 0.0, 0.0),
                );
                // this is awful
                f.add_body_part(
                    physics_world
                        .collider_body_handle(physics_bodies.get(entity).unwrap().0)
                        .unwrap(),
                );
                (entity, physics_world.add_force_generator(f), toi)
            })
            .map(|(entity, antig, toi)| {
                let rot_inv = physics_world
                    .rigid_body(
                        physics_world
                            .collider_body_handle(physics_bodies.get(entity).unwrap().0)
                            .unwrap(),
                    )
                    .unwrap()
                    .position()
                    .rotation
                    .inverse();
                SelectedObject {
                    entity,
                    previous_camera_position: camera_isometry,
                    force: antig,
                    distance: toi,
                    box_forward: rot_inv * (camera_isometry.rotation * Vector3::z()),
                    box_up: rot_inv * (camera_isometry.rotation * Vector3::y()),
                }
            });
    }

    fn drop_object(&mut self, physics_world: &mut Write<MyWorld>) {
        if let Some(ref so) = self.selected_object {
            physics_world.remove_force_generator(so.force);
        }
        self.selected_object = None;
    }
}

type PointingSystemData<'s> = (
    Entities<'s>,
    ReadStorage<'s, Camera>,
    Write<'s, MyWorld>,
    ReadStorage<'s, Transform>,
    WriteStorage<'s, PhysicsBody>,
    Read<'s, InputHandler<String, String>>,
    ReadStorage<'s, Grabbable>,
    Read<'s, Time>,
    Read<'s, EventChannel<Event>>,
);

impl<'s> System<'s> for PointingSystem {
    type SystemData = PointingSystemData<'s>;
    fn run(
        &mut self,
        (
            entities,
            cameras,
            mut physics_world,
            transforms,
            physics_bodies,
            input,
            grabbables,
            time,
            events,
        ): Self::SystemData,
    ) {
        for event in events.read(
            &mut self
                .event_reader
                .as_mut()
                .expect("`PointingSystem::setup` was not called before `PointingSystem::run`"),
        ) {
            if input.mouse_button_is_down(MouseButton::Right) {
                if let Event::DeviceEvent { ref event, .. } = *event {
                    if let DeviceEvent::MouseMotion { delta: (x, y) } = *event {
                        if let Some(ref mut so) = self.selected_object {
                            let q = UnitQuaternion::from_axis_angle(
                                &Vector3::y_axis(),
                                (-x as f32 * 0.2).to_radians(),
                            )
                            .inverse()
                                * UnitQuaternion::from_axis_angle(
                                    &Vector3::x_axis(),
                                    (-y as f32 * 0.2).to_radians(),
                                )
                                .inverse();
                            so.box_forward = q * so.box_forward;
                            so.box_up = q * so.box_up;
                        }
                    }
                }
            }
        }
        let is_left_click = input.mouse_button_is_down(MouseButton::Left);
        match (
            is_left_click,
            self.selected_object.is_some(),
            self.did_release_click,
        ) {
            (true, true, false) | (false, true, _) => {
                self.move_selected_object(
                    &cameras,
                    &transforms,
                    &physics_bodies,
                    &mut physics_world,
                    &time,
                );
                self.did_release_click = !is_left_click;
            }
            (true, false, true) => {
                self.did_release_click = false;
                self.grab_object(
                    &entities,
                    &cameras,
                    &mut physics_world,
                    &transforms,
                    &physics_bodies,
                    &grabbables,
                );
            }
            (true, true, true) => {
                self.did_release_click = false;
                self.drop_object(&mut physics_world);
            }
            (true, false, false) => (),
            (false, false, _) => self.did_release_click = true,
        }
    }

    fn setup(&mut self, res: &mut Resources) {
        use amethyst::core::specs::prelude::SystemData;

        Self::SystemData::setup(res);
        self.event_reader = Some(res.fetch_mut::<EventChannel<Event>>().register_reader());
    }
}
