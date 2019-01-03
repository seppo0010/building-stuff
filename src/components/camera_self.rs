use amethyst::ecs::{Component, FlaggedStorage};

pub struct CameraSelf;
impl Component for CameraSelf {
    type Storage = FlaggedStorage<Self>;
}