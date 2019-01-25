use nphysics3d::world::World as PhysicsWorld;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, RwLock};

type MyCollisionWorld = Box<Arc<RwLock<PhysicsWorld<f32>>>>;
pub struct MyWorld {
    pub inner: MyCollisionWorld,
}

impl Default for MyWorld {
    fn default() -> Self {
        MyWorld {
            inner: Box::new(Arc::new(RwLock::new(PhysicsWorld::default()))),
        }
    }
}

impl MyWorld {
    pub fn get<'a: 'b, 'b>(&'a self) -> Box<Deref<Target = PhysicsWorld<f32>> + 'b> {
        Box::new(self.inner.read().unwrap())
    }

    pub fn get_mut<'a: 'b, 'b>(&'a mut self) -> Box<DerefMut<Target = PhysicsWorld<f32>> + 'b> {
        Box::new(self.inner.write().unwrap())
    }
}
