use crate::resources::MyWorld;

use amethyst::{
    core::{
        nalgebra::{Unit, Vector3},
        Transform,
    },
    ecs::{Join, Read, ReadStorage, System, Write},
    input::InputHandler,
    renderer::Camera,
};
use f32;
use na::Isometry3;
use ncollide3d::{
    bounding_volume::{HasBoundingVolume, AABB},
    shape::{ConvexHull, Cuboid, Cylinder, ShapeHandle},
    transformation::ToTriMesh,
    world::CollisionObjectHandle,
};
use nphysics3d::{
    force_generator::ConstantAcceleration, object::SensorHandle, volumetric::Volumetric,
};
use winit::VirtualKeyCode;

const MERGE_DEPTH: f32 = 5.0;

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
        let camera_transform = (cameras, transforms).join().next().unwrap().1;

        // copy paste much? Yes.
        let cylinder = Cylinder::new(MERGE_DEPTH, 0.75);
        let aabb: AABB<f32> = cylinder.bounding_volume(&Isometry3::identity());
        let t = cylinder.to_trimesh(10);
        let geom = ShapeHandle::new(ConvexHull::try_from_points(&t.coords).unwrap());
        let inertia = Cuboid::new(aabb.half_extents()).inertia(1.0);
        let center_of_mass = aabb.center();

        let mut t = camera_transform.clone();
        t.pitch_global(f32::consts::FRAC_PI_2);
        if let Some(dir) = Unit::try_new(Vector3::x(), 1.0e-6) {
            let d = t.isometry().rotation * dir.as_ref();
            if let Some(d) = Unit::try_new(Vector3::new(d.x, 0.0, d.z), 1.0e-6) {
                let linear = Vector3::new(d.x, 0.0, d.z);
                *t.translation_mut() += linear * -MERGE_DEPTH;
            }
        }

        let handle = physics_world.add_rigid_body(*t.isometry(), inertia, center_of_mass);

        let mut f =
            ConstantAcceleration::new(-physics_world.gravity(), Vector3::new(0.0, 0.0, 0.0));
        f.add_body_part(handle);
        physics_world.add_force_generator(f);

        self.mysensor = Some(physics_world.add_sensor(geom, handle, Isometry3::identity()));
    }

    fn remove_sensor<'s>(&mut self, physics_world: &mut Write<'s, MyWorld>) {
        if let Some(mysensor) = self.mysensor {
            println!("removing sensor");
            physics_world.remove_colliders(&[mysensor]);
            self.mysensor = None;
        }
    }

    fn log_sensor<'s>(&mut self, physics_world: &Write<'s, MyWorld>) {
        if let Some(mysensor) = self.mysensor {
            for (o1, o2, g) in physics_world.collision_world().contact_pairs() {
                if o1.handle() != CollisionObjectHandle(0) {
                    println!(
                        "{:?} {:?} {:?} {:?}",
                        o1.handle(),
                        o2.handle(),
                        g.num_contacts(),
                        mysensor
                    );
                }
            }
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
                self.log_sensor(&physics_world);
            } else {
                self.create_sensor(&mut physics_world, &cameras, &transforms);
            }
        } else {
            self.remove_sensor(&mut physics_world);
        }
    }
}
