use std::f32;

use crate::{
    components::{CameraSelf, Grabbable, PhysicsBody},
    resources::MyWorld,
};

use amethyst::{
    assets::{Loader, ProgressCounter},
    core::{
        nalgebra::{UnitQuaternion, Vector3},
        Transform,
    },
    prelude::*,
    renderer::{
        AmbientColor, Camera, DirectionalLight, Light, Material, MaterialDefaults, MeshHandle,
        PosNormTex, Projection, Rgba, Shape,
    },
    ui::UiCreator,
    utils::application_root_dir,
};

use na::{Isometry3, Point3, Vector3 as PhysicsVector3};

use ncollide3d::{
    bounding_volume::{HasBoundingVolume, AABB},
    shape::{Cuboid, Cylinder, ShapeHandle, TriMesh},
    transformation::ToTriMesh,
};
use nphysics3d::{
    object::{BodyHandle, BodyStatus, Material as PhysicsMaterial},
    volumetric::Volumetric,
};
const COLLIDER_MARGIN: f32 = 0.01;

#[derive(Default)]
pub struct GameState {
    pub cube_mesh: Option<MeshHandle>,
    pub cube_materials: Vec<Material>,
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
        let tex_storage = world.read_resource();
        let mut progress = ProgressCounter::default();
        let loader = world.read_resource::<Loader>();
        let mesh_data = Shape::Cube.generate::<Vec<PosNormTex>>(None);
        self.cube_mesh = Some(loader.load_from_data(mesh_data, &mut progress, &mesh_storage));
        for color in [
            [0.0, 1.0, 0.0, 1.0],
            [1.0, 1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
        ]
        .into_iter()
        {
            self.cube_materials.push(Material {
                albedo: loader.load_from_data((*color).into(), &mut progress, &tex_storage),
                ..world.read_resource::<MaterialDefaults>().0.clone()
            });
        }
    }

    fn create_cube(&mut self, world: &mut World, i: usize, physics_world: &mut MyWorld) {
        let mut t = Transform::default();
        *t.scale_mut() = Vector3::new(0.5, 0.5, 0.5);
        *t.translation_mut() = Vector3::new(
            (i as f32) * 3.0 - 7.5,
            (i as f32) * 3.0 + 2.5,
            3.0 + (-1.0_f32).powf(i as f32) * 0.5,
        );

        let geom = ShapeHandle::new(Cuboid::new(PhysicsVector3::repeat(0.5 - COLLIDER_MARGIN)));
        let inertia = geom.inertia(1.0);
        let center_of_mass = geom.center_of_mass();

        let pos = Isometry3::new(
            {
                let translation = t.translation();
                PhysicsVector3::new(translation[0], translation[1], translation[2])
            },
            Vector3::new(0.9, 0.1, 0.0),
        );
        let handle = physics_world.add_rigid_body(pos, inertia, center_of_mass);

        let body_handle = physics_world.add_collider(
            COLLIDER_MARGIN,
            geom.clone(),
            handle,
            Isometry3::identity(),
            PhysicsMaterial::default(),
        );

        world
            .create_entity()
            .named(format!("box{}", i))
            .with(t)
            .with(self.cube_mesh.clone().unwrap())
            .with(self.cube_materials[i].clone())
            .with(PhysicsBody(body_handle))
            .with(Grabbable)
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

        let geom = ShapeHandle::new(Cuboid::new(PhysicsVector3::new(1000.0, 0.0, 1000.0)));

        physics_world.add_collider(
            COLLIDER_MARGIN,
            geom.clone(),
            BodyHandle::ground(),
            Isometry3::new(PhysicsVector3::new(0.0, 0.0, 0.0), na::zero()),
            PhysicsMaterial::default(),
        );

        world
            .create_entity()
            .named("floor")
            .with(plane)
            .with(color)
            .with(t)
            .build();
    }

    fn create_self(&mut self, world: &mut World, physics_world: &mut MyWorld) {
        // this is a bit strange, but ncollide has two different TriMesh that are quite similar
        let cylinder = Cylinder::new(0.9, 0.75);
        let mut t = cylinder.to_trimesh(10);
        t.unify_index_buffer();
        let aabb: AABB<f32> = cylinder.bounding_volume(&Isometry3::identity());
        let geom = ShapeHandle::new(TriMesh::new(
            t.coords,
            t.indices
                .unwrap_unified()
                .into_iter()
                .map(|p| {
                    Point3::new(
                        p.coords.x as usize,
                        p.coords.y as usize,
                        p.coords.z as usize,
                    )
                })
                .collect(),
            t.uvs,
        ));
        let inertia = Cuboid::new(aabb.half_extents()).inertia(1.0);
        let center_of_mass = aabb.center();

        let pos = Isometry3::new(
            PhysicsVector3::new(0.0, 0.9, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        );
        let handle = physics_world.add_rigid_body(pos, inertia, center_of_mass);
        physics_world
            .rigid_body_mut(handle)
            .unwrap()
            .set_status(BodyStatus::Kinematic);

        let body_handle = physics_world.add_collider(
            COLLIDER_MARGIN,
            geom.clone(),
            handle,
            Isometry3::identity(),
            PhysicsMaterial::default(),
        );

        world
            .create_entity()
            .named("self")
            .with(PhysicsBody(body_handle))
            .with(CameraSelf)
            .build();
    }
    fn create_camera(&mut self, world: &mut World) {
        let mut t = Transform::default();
        *t.translation_mut() = Vector3::new(0.0, 1.8, 0.0);
        *t.rotation_mut() = UnitQuaternion::new_observer_frame(
            &Vector3::new(0.0, 0.0, -1.0),
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
            let app_root = application_root_dir();
            creator.create(format!("{}/resources/hud.ron", app_root), ());
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
        physics_world.step();
        physics_world.set_gravity(-PhysicsVector3::y() * 9.81);
        self.create_self(data.world, &mut physics_world);
        self.create_camera(data.world);
        self.create_center(data.world);

        // let mut testbed = nphysics_testbed3d::Testbed::new(physics_world.inner);
        // testbed.look_at(Point3::new(-4.0, 1.0, -4.0), Point3::new(0.0, 1.0, 0.0));
        // testbed.run();

        data.world.add_resource(physics_world);
    }
}
