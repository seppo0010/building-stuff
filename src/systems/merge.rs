use crate::resources::MyWorld;

use amethyst::{
    core::Transform,
    ecs::{Join, Read, ReadStorage, System, Write},
    input::InputHandler,
    renderer::Camera,
};
use na::Isometry3;
use ncollide3d::{
    bounding_volume::{HasBoundingVolume, AABB},
    shape::{ConvexHull, Cuboid, Cylinder, ShapeHandle},
    transformation::ToTriMesh,
};
use nphysics3d::{object::SensorHandle, volumetric::Volumetric};
use winit::VirtualKeyCode;

#[derive(Default)]
pub struct MergeSystem {
    mysensor: Option<SensorHandle>,
}

impl MergeSystem {
    fn create_sensor<'s>(
        &mut self,
        physics_world: &mut Write<'s, MyWorld>,
        cameras: &ReadStorage<'s, Camera>,
        transforms: &ReadStorage<'s, Transform>,
    ) {
        println!("adding sensor");
        let camera_isometry = (cameras, transforms).join().next().unwrap().1.isometry();

        // copy paste much? Yes.
        let cylinder = Cylinder::new(10.0 / 2.0, 0.75);
        let aabb: AABB<f32> = cylinder.bounding_volume(&Isometry3::identity());
        let t = cylinder.to_trimesh(10);
        let geom = ShapeHandle::new(ConvexHull::try_from_points(&t.coords).unwrap());
        let inertia = Cuboid::new(aabb.half_extents()).inertia(1.0);
        let center_of_mass = aabb.center();

        let handle = physics_world.add_rigid_body(*camera_isometry, inertia, center_of_mass);
        self.mysensor = Some(physics_world.add_sensor(geom, handle, Isometry3::identity()));
    }

    fn remove_sensor<'s>(&mut self, physics_world: &mut Write<'s, MyWorld>) {
        if let Some(mysensor) = self.mysensor {
            println!("removing sensor");
            physics_world.remove_colliders(&[mysensor]);
            self.mysensor = None;
        }
    }
}

impl<'s> System<'s> for MergeSystem {
    type SystemData = (
        Write<'s, MyWorld>,
        Read<'s, InputHandler<String, String>>,
        ReadStorage<'s, Camera>,
        ReadStorage<'s, Transform>,
    );
    fn run(&mut self, (mut physics_world, input, cameras, transforms): Self::SystemData) {
        if input.key_is_down(VirtualKeyCode::E) {
            if self.mysensor.is_some() {
                return;
            }
            self.create_sensor(&mut physics_world, &cameras, &transforms);
        } else {
            self.remove_sensor(&mut physics_world);
        }
    }
}
