use amethyst::ecs::{Component, FlaggedStorage};

pub struct Grabbable;
impl Component for Grabbable {
    type Storage = FlaggedStorage<Self>;
}
