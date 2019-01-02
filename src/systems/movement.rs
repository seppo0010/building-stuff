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
    shrev::{EventChannel, ReaderId},
};
use specs::prelude::Resources;
use winit::{DeviceEvent, Event};
pub struct MovementSystem {
    sensitivity_x: f32,
    sensitivity_y: f32,
    speed: f32,
    event_reader: Option<ReaderId<Event>>,
}

impl Default for MovementSystem {
    fn default() -> Self {
        MovementSystem {
            sensitivity_x: 0.2,
            sensitivity_y: 0.2,
            speed: 1.0,
            event_reader: None,
        }
    }
}

type MovementSystemData<'s> = (
    Read<'s, EventChannel<Event>>,
    WriteStorage<'s, Transform>,
    ReadStorage<'s, Camera>,
    Read<'s, WindowFocus>,
    Read<'s, HideCursor>,
    Read<'s, Time>,
    Read<'s, InputHandler<String, String>>,
);
impl<'s> System<'s> for MovementSystem {
    type SystemData = MovementSystemData<'s>;

    fn run(
        &mut self,
        (events, mut transforms, cameras, focus, hide, time, input): Self::SystemData,
    ) {
        if focus.is_focused && hide.hide {
            let x = get_input_axis_simple(&Some("move_x".to_owned()), &input);
            let z = get_input_axis_simple(&Some("move_z".to_owned()), &input);
            if let Some(dir) = Unit::try_new(Vector3::new(x, 0.0, z), 1.0e-6) {
                for (transform, _) in (&mut transforms, &cameras).join() {
                    let mut iso = transform.isometry_mut();
                    let d = iso.rotation * dir.as_ref();
                    let total = d.x.abs() + d.z.abs();
                    iso.translation.vector += Vector3::new(d.x / total, 0.0, d.z / total)
                        * time.delta_seconds()
                        * self.speed;
                }
            }
            for event in events.read(
                &mut self
                    .event_reader
                    .as_mut()
                    .expect("`MovementSystem::setup` was not called before `MovementSystem::run`"),
            ) {
                if let Event::DeviceEvent { ref event, .. } = *event {
                    if let DeviceEvent::MouseMotion { delta: (x, y) } = *event {
                        for (transform, _) in (&mut transforms, &cameras).join() {
                            transform.pitch_local((-y as f32 * self.sensitivity_y).to_radians());
                            transform.yaw_global((-x as f32 * self.sensitivity_x).to_radians());
                            // there's probably a better way to do this if you know trigonometry :see_no_evil:
                            while (transform.isometry().rotation * Vector3::z()).y < -0.8 {
                                transform.pitch_local((-1.0_f32).to_radians());
                            }
                            while (transform.isometry().rotation * Vector3::z()).y > 0.8 {
                                transform.pitch_local((1.0_f32).to_radians());
                            }
                        }
                    }
                }
            }
        }
    }

    fn setup(&mut self, res: &mut Resources) {
        use amethyst::core::specs::prelude::SystemData;

        Self::SystemData::setup(res);
        self.event_reader = Some(res.fetch_mut::<EventChannel<Event>>().register_reader());
    }
}
