use amethyst::{
    ecs::{Component, VecStorage},
    renderer::Material,
};

pub struct Grabbable {
    pub default_material: Material,
    pub selected_material: Material,
}

impl Component for Grabbable {
    type Storage = VecStorage<Self>;
}
