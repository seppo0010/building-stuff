use amethyst::ecs::{Component, VecStorage};
use ncollide3d::world::CollisionObjectSlabHandle;

pub struct PhysicsBody(pub CollisionObjectSlabHandle);
impl Component for PhysicsBody {
    type Storage = VecStorage<Self>;
}
