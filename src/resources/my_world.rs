use nphysics3d::world::World as PhysicsWorld;
use std::ops::{Deref, DerefMut};

type MyCollisionWorld = PhysicsWorld<f32>;
pub struct MyWorld {
    inner: MyCollisionWorld,
}

impl Default for MyWorld {
    fn default() -> Self {
        MyWorld {
            inner: MyCollisionWorld::new(),
        }
    }
}

impl Deref for MyWorld {
    type Target = MyCollisionWorld;

    fn deref(&self) -> &MyCollisionWorld {
        &self.inner
    }
}

impl DerefMut for MyWorld {
    fn deref_mut(&mut self) -> &mut MyCollisionWorld {
        &mut self.inner
    }
}
