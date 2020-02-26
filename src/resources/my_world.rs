use nphysics3d::world::MechanicalWorld as PhysicsWorld;

use std::ops::{Deref, DerefMut};

type MyCollisionWorld = PhysicsWorld<f32>;
pub struct MyWorld {
    pub inner: MyCollisionWorld,
    force_generators: DefaultForceGeneratorSet,
}

impl MyWorld {
    pub fn add_force_generator(
        &mut self,
        force_generator: ConstantAcceleration,
    ) -> DefaultForceGeneratorHandle {
        self.force_generators.insert(force_generator)
    }
}

impl Default for MyWorld {
    fn default() -> Self {
        MyWorld {
            inner: MyCollisionWorld::new(),
            force_generators: Vec::new(),
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
