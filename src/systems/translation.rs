use std::f32;

use crate::{
    components::{CameraSelf, PhysicsBody},
    resources::MyWorld,
};
use amethyst::{
    controls::{HideCursor, WindowFocus},
    core::{
        nalgebra::{Unit, Vector3},
        Transform,
    },
    ecs::{Join, Read, ReadStorage, System, Write, WriteStorage},
    input::{get_input_axis_simple, InputHandler},
    renderer::Camera,
};
pub struct TranslationSystem {
    speed: f32,
    speed_running: f32,
}

impl Default for TranslationSystem {
    fn default() -> Self {
        TranslationSystem {
            speed: 1.0,
            speed_running: 5.0,
        }
    }
}

type TranslationSystemData<'s> = (
    WriteStorage<'s, Transform>,
    ReadStorage<'s, Camera>,
    Read<'s, WindowFocus>,
    Read<'s, HideCursor>,
    Read<'s, InputHandler<String, String>>,
    ReadStorage<'s, CameraSelf>,
    Write<'s, MyWorld>,
    WriteStorage<'s, PhysicsBody>,
);
impl<'s> System<'s> for TranslationSystem {
    type SystemData = TranslationSystemData<'s>;

    fn run(
        &mut self,
        (
            mut transforms,
            cameras,
            focus,
            hide,
            input,
            cameraself,
            mut physics_world,
            mut physics_body,
        ): Self::SystemData,
    ) {
        let mut world = physics_world.get_mut();
        for (_, body) in (&cameraself, &mut physics_body).join() {
            if let Some(ref mut rb) = world
                .collider_body_handle(body.0)
                .and_then(|bh| world.rigid_body_mut(bh))
            {
                rb.set_linear_velocity(Vector3::new(0.0, 0.0, 0.0));
                rb.set_angular_velocity(Vector3::new(0.0, 0.0, 0.0));
            }
            if focus.is_focused && hide.hide {
                let x = get_input_axis_simple(&Some("move_x".to_owned()), &input);
                let z = get_input_axis_simple(&Some("move_z".to_owned()), &input);
                if let Some(dir) = Unit::try_new(Vector3::new(x, 0.0, z), 1.0e-6) {
                    for (transform, _) in (&mut transforms, &cameras).join() {
                        let mut iso = transform.isometry_mut();
                        let d = iso.rotation * dir.as_ref();
                        if let Some(d) = Unit::try_new(Vector3::new(d.x, 0.0, d.z), 1.0e-6) {
                            let linear = Vector3::new(d.x, 0.0, d.z);
                            let h = if let Some(co) = world.collider(body.0) {
                                co.shape().aabb(co.position()).maxs().y
                            } else {
                                0.0
                            };
                            if let Some(rb) = world
                                .collider_body_handle(body.0)
                                .and_then(|bh| world.rigid_body_mut(bh))
                            {
                                let speed = if input.key_is_down(winit::VirtualKeyCode::LShift) {
                                    self.speed_running
                                } else {
                                    self.speed
                                };
                                rb.set_linear_velocity(linear * speed);
                                let pos = rb.position().translation.vector;
                                iso.translation.vector = Vector3::new(pos.x, h, pos.z);
                            }
                        }
                    }
                }
            }
        }
    }
}
