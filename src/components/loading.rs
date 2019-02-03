use amethyst::ecs::{Component, FlaggedStorage};

#[derive(Default)]
pub struct Loading;
impl Component for Loading {
    type Storage = FlaggedStorage<Self>;
}