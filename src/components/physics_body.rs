use amethyst::ecs::{Component, VecStorage};
use ncollide3d::world::CollisionObjectHandle;

pub struct PhysicsBody(pub CollisionObjectHandle);
impl Component for PhysicsBody {
    type Storage = VecStorage<Self>;
}
