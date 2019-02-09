#[cfg(feature = "testbed")]
use amethyst::core::nalgebra::Point3;
use std::f32;
#[cfg(feature = "testbed")]
use std::thread;

use crate::{
    components::{CameraSelf, Grabbable, Mergeable, PhysicsBody},
    resources::MyWorld,
};

use amethyst::{
    assets::{AssetStorage, Loader, ProgressCounter},
    core::{
        nalgebra::{Isometry3, UnitQuaternion, Vector3},
        Transform,
    },
    prelude::*,
    renderer::{
        AmbientColor, Camera, DirectionalLight, Light, Material, MaterialDefaults, MeshHandle,
        PosNormTex, Projection, Rgba, Shape, Texture,
    },
    ui::UiCreator,
    utils::application_root_dir,
};

use ncollide3d::{
    shape::{ConvexHull, Cuboid, Cylinder, ShapeHandle},
    transformation::ToTriMesh,
};
use nphysics3d::object::{BodyStatus, ColliderDesc, RigidBodyDesc};

const COLLIDER_MARGIN: f32 = 0.01;
const CAMERA_HEIGHT: f32 = 1.8;
const INITIAL_CAMERA_X: f32 = 8.0;
const INITIAL_CAMERA_Z: f32 = 4.0;
const COLORS: [[f32; 4]; 5] = [
    [0.0, 1.0, 0.0, 1.0],
    [1.0, 1.0, 0.0, 1.0],
    [1.0, 0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0, 1.0],
    [0.0, 0.0, 1.0, 1.0],
];

#[derive(Default)]
pub struct GameState {
    pub cube_mesh: Option<MeshHandle>,
}

impl GameState {
    fn create_light(&mut self, world: &mut World) {
        world.add_resource(AmbientColor(Rgba(0.3, 0.3, 0.3, 1.0)));
        for (dir, pos, color) in [
            ([-1.0, 0.0, 0.0], [100.0, 0.0, 0.0], 0.08_f32),
            ([1.0, 0.0, 0.0], [-100.0, 0.0, 0.0], 0.09_f32),
            ([0.0, 0.0, -1.0], [0.0, 0.0, 100.0], 0.10_f32),
            ([0.0, 0.0, 1.0], [0.0, 0.0, -100.0], 0.11_f32),
            ([0.0, -1.0, 0.0], [0.0, 100.0, 0.0], 0.12_f32),
            ([0.3, -1.0, 0.3], [0.0, 100.0, 10.0], 0.6f32),
        ]
        .into_iter()
        {
            let mut s = DirectionalLight::default();
            s.direction = *dir;
            s.color = Rgba(*color, *color, *color, 1.0);
            let mut t = Transform::default();
            *t.translation_mut() = Vector3::new(pos[0], pos[1], pos[2]);

            world
                .create_entity()
                .with(t)
                .with(Light::Directional(s))
                .build();
        }
    }

    fn prepare_cubes(&mut self, world: &mut World) {
        let mesh_storage = world.read_resource();
        let mut progress = ProgressCounter::default();
        let loader = world.read_resource::<Loader>();
        let mesh_data = Shape::Cube.generate::<Vec<PosNormTex>>(Some((0.5, 0.5, 0.5)));
        self.cube_mesh = Some(loader.load_from_data(mesh_data, &mut progress, &mesh_storage));
    }

    fn create_cube(&mut self, world: &mut World, i: usize, physics_world: &mut MyWorld) {
        let name = format!("box{}", i);
        let collider = ColliderDesc::new(ShapeHandle::new(Cuboid::new(Vector3::repeat(
            0.5 - COLLIDER_MARGIN,
        ))))
        .density(1.0)
        .name(name.clone());

        let _ = RigidBodyDesc::new()
            .translation(Vector3::new(
                (i as f32) * 3.0 - 7.5,
                (i as f32) * 3.0 + 2.5,
                3.0 + (-1.0_f32).powf(i as f32) * 0.5,
            ))
            .rotation(Vector3::new(0.9, 0.1, 0.0))
            .name(name.clone())
            .collider(&collider)
            .build(&mut physics_world.get_mut());

        let grabbable = {
            let loader = world.read_resource::<Loader>();
            let tex_storage = world.read_resource::<AssetStorage<Texture>>();
            Grabbable {
                default_material: Material {
                    albedo: loader.load_from_data(COLORS[i].into(), (), &tex_storage),
                    ..world.read_resource::<MaterialDefaults>().0.clone()
                },
                selected_material: Material {
                    metallic: loader.load_from_data(COLORS[i].into(), (), &tex_storage),
                    ..world.read_resource::<MaterialDefaults>().0.clone()
                },
            }
        };

        let pw = physics_world.get();
        world
            .create_entity()
            .named(name.clone())
            .with(Transform::default())
            .with(self.cube_mesh.clone().unwrap())
            .with(Mergeable)
            .with(grabbable.default_material.clone())
            .with(PhysicsBody(
                pw.collider_world()
                    .colliders_with_name(&name)
                    .next()
                    .unwrap()
                    .handle(),
            ))
            .with(grabbable)
            .build();
    }

    fn create_floor(&mut self, world: &mut World, physics_world: &mut MyWorld) {
        let mut t = Transform::default();
        *t.rotation_mut() = UnitQuaternion::new(Vector3::new(0.0, 1.0, 0.0));
        *t.scale_mut() = Vector3::new(1000.0, 0.0, 1000.0);
        *t.translation_mut() = Vector3::new(0.0, 0.0, 0.0);

        let (plane, color) = {
            let mesh_storage = world.read_resource();
            let tex_storage = world.read_resource();
            let mut progress = ProgressCounter::default();
            let loader = world.read_resource::<Loader>();
            let mesh_data = Shape::Cube.generate::<Vec<PosNormTex>>(None);
            let plane: MeshHandle = loader.load_from_data(mesh_data, &mut progress, &mesh_storage);
            let color = Material {
                albedo: loader.load_from_data(
                    [135.0 / 255.0, 67.0 / 255.0, 23.0 / 255.0, 1.0].into(),
                    &mut progress,
                    &tex_storage,
                ),
                ..world.read_resource::<MaterialDefaults>().0.clone()
            };
            (plane, color)
        };

        let name = "floor".to_owned();
        let collider = ColliderDesc::new(ShapeHandle::new(Cuboid::new(Vector3::new(
            1000.0, 0.0, 1000.0,
        ))))
        .density(1.0)
        .name(name.clone());

        let _ = RigidBodyDesc::new()
            .name(name.clone())
            .collider(&collider)
            .build(&mut physics_world.get_mut());

        world
            .create_entity()
            .named(name)
            .with(plane)
            .with(color)
            .with(t)
            .build();
    }

    fn create_self(&mut self, world: &mut World, physics_world: &mut MyWorld) {
        let cylinder = Cylinder::new(CAMERA_HEIGHT / 2.0, 0.75);
        let t = cylinder.to_trimesh(10);

        let name = "self".to_owned();
        let collider = ColliderDesc::new(ShapeHandle::new(
            ConvexHull::try_from_points(&t.coords).unwrap(),
        ))
        .density(1.0)
        .name(name.clone());

        let _ = RigidBodyDesc::new()
            .name(name.clone())
            .position(Isometry3::new(
                Vector3::new(INITIAL_CAMERA_X, CAMERA_HEIGHT / 2.0, INITIAL_CAMERA_Z),
                Vector3::new(0.0, 1.0, 0.0),
            ))
            .status(BodyStatus::Kinematic)
            .collider(&collider)
            .build(&mut physics_world.get_mut());

        let pw = physics_world.get();
        world
            .create_entity()
            .named(name.clone())
            .with(PhysicsBody(
                pw.collider_world()
                    .colliders_with_name(&name)
                    .next()
                    .unwrap()
                    .handle(),
            ))
            .with(CameraSelf)
            .build();
    }
    fn create_camera(&mut self, world: &mut World) {
        let mut t = Transform::default();
        *t.translation_mut() = Vector3::new(INITIAL_CAMERA_X, CAMERA_HEIGHT, INITIAL_CAMERA_Z);
        *t.rotation_mut() = UnitQuaternion::face_towards(
            &Vector3::new(1.0, 0.0, 0.0),
            &Vector3::new(0.0, 1.0, 0.0),
        );
        let c = Camera::from(Projection::perspective(1.3, f32::consts::FRAC_PI_3));
        world
            .create_entity()
            .named("camera")
            .with(c)
            .with(t)
            .build();
    }

    fn create_center(&mut self, world: &mut World) {
        world.exec(|mut creator: UiCreator| {
            let app_root = application_root_dir().unwrap();
            creator.create(
                format!("{}/resources/hud.ron", app_root.to_str().unwrap()),
                (),
            );
        });
    }
}
impl SimpleState for GameState {
    fn on_start(&mut self, data: StateData<GameData>) {
        data.world.register::<PhysicsBody>();
        data.world.register::<CameraSelf>();
        let mut physics_world = MyWorld::default();
        self.create_light(data.world);
        self.create_floor(data.world, &mut physics_world);
        self.prepare_cubes(data.world);
        for i in 0..5 {
            self.create_cube(data.world, i, &mut physics_world);
        }
        physics_world.get_mut().step();
        physics_world.get_mut().set_gravity(-Vector3::y() * 9.81);
        self.create_self(data.world, &mut physics_world);
        self.create_camera(data.world);
        self.create_center(data.world);

        #[cfg(feature = "testbed")]
        {
            let world_copy = physics_world.inner.clone();
            thread::spawn(move || {
                let mut testbed = nphysics_testbed3d::Testbed::new_with_world_owner(world_copy);
                testbed.hide_performance_counters();
                testbed.look_at(Point3::new(-4.0, 1.0, -4.0), Point3::new(0.0, 1.0, 0.0));
                testbed.run();
            });
        }
        data.world.add_resource(physics_world);
    }
}
