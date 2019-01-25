use crate::resources::MyWorld;

use amethyst::{
    core::{
        Transform,
    },
    ecs::{Read, ReadStorage, System, Write},
    input::InputHandler,
    renderer::Camera,
};
use winit::VirtualKeyCode;

const MERGE_DEPTH: f32 = 5.0;

#[derive(Default)]
pub struct MergeSystem;

impl MergeSystem {
}

impl<'s> System<'s> for MergeSystem {
    type SystemData = (
        Write<'s, MyWorld>,
        Read<'s, InputHandler<String, String>>,
        ReadStorage<'s, Camera>,
        ReadStorage<'s, Transform>,
    );
    fn run(&mut self, (mut physics_world, input, cameras, transforms): Self::SystemData) {
        if input.key_is_down(VirtualKeyCode::E) {
        } else {
        }
    }
}
