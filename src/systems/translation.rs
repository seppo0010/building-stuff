use std::f32;

use amethyst::{
    controls::{HideCursor, WindowFocus},
    core::{
        nalgebra::{Unit, Vector3},
        timing::Time,
        Transform,
    },
    ecs::{Join, Read, ReadStorage, System, WriteStorage},
    input::{get_input_axis_simple, InputHandler},
    renderer::Camera,
};
pub struct TranslationSystem {
    speed: f32,
}

impl Default for TranslationSystem {
    fn default() -> Self {
        TranslationSystem { speed: 1.0 }
    }
}

type TranslationSystemData<'s> = (
    WriteStorage<'s, Transform>,
    ReadStorage<'s, Camera>,
    Read<'s, WindowFocus>,
    Read<'s, HideCursor>,
    Read<'s, Time>,
    Read<'s, InputHandler<String, String>>,
);
impl<'s> System<'s> for TranslationSystem {
    type SystemData = TranslationSystemData<'s>;

    fn run(&mut self, (mut transforms, cameras, focus, hide, time, input): Self::SystemData) {
        if focus.is_focused && hide.hide {
            let x = get_input_axis_simple(&Some("move_x".to_owned()), &input);
            let z = get_input_axis_simple(&Some("move_z".to_owned()), &input);
            if let Some(dir) = Unit::try_new(Vector3::new(x, 0.0, z), 1.0e-6) {
                for (transform, _) in (&mut transforms, &cameras).join() {
                    let mut iso = transform.isometry_mut();
                    let d = iso.rotation * dir.as_ref();
                    if let Some(d) = Unit::try_new(Vector3::new(d.x, 0.0, d.z), 1.0e-6) {
                        iso.translation.vector +=
                            Vector3::new(d.x, 0.0, d.z) * time.delta_seconds() * self.speed;
                    }
                }
            }
        }
    }
}
