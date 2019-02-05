use std::f32;

use amethyst::{
    controls::{HideCursor, WindowFocus},
    core::{nalgebra::Vector3, Transform},
    ecs::{Join, Read, ReadStorage, System, WriteStorage},
    input::InputHandler,
    renderer::{Camera, MouseButton},
    shrev::{EventChannel, ReaderId},
};
use specs::prelude::Resources;
use winit::{DeviceEvent, Event};
pub struct RotationSystem {
    sensitivity_x: f32,
    sensitivity_y: f32,
    event_reader: Option<ReaderId<Event>>,
}

impl Default for RotationSystem {
    fn default() -> Self {
        RotationSystem {
            sensitivity_x: 0.2,
            sensitivity_y: 0.2,
            event_reader: None,
        }
    }
}

impl RotationSystem {
    fn rotate<'s>(
        &self,
        transforms: &mut WriteStorage<'s, Transform>,
        cameras: &ReadStorage<'s, Camera>,
        x: f32,
        y: f32,
    ) {
        for (transform, _) in (transforms, cameras).join() {
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

type RotationSystemData<'s> = (
    Read<'s, EventChannel<Event>>,
    WriteStorage<'s, Transform>,
    ReadStorage<'s, Camera>,
    Read<'s, WindowFocus>,
    Read<'s, HideCursor>,
    Read<'s, InputHandler<String, String>>,
);

impl<'s> System<'s> for RotationSystem {
    type SystemData = RotationSystemData<'s>;

    fn run(&mut self, (events, mut transforms, cameras, focus, hide, input): Self::SystemData) {
        for event in events.read(
            &mut self
                .event_reader
                .as_mut()
                .expect("`RotationSystem::setup` was not called before `RotationSystem::run`"),
        ) {
            if !input.mouse_button_is_down(MouseButton::Right) && focus.is_focused && hide.hide {
                if let Event::DeviceEvent { ref event, .. } = *event {
                    if let DeviceEvent::MouseMotion { delta: (x, y) } = *event {
                        self.rotate(&mut transforms, &cameras, x as f32, y as f32);
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
