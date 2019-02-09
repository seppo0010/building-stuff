use amethyst::ecs::{Component, VecStorage};

pub struct Mergeable;
impl Component for Mergeable {
    type Storage = VecStorage<Self>;
}
