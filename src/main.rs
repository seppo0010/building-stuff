extern crate amethyst;
extern crate nalgebra as na;
extern crate ncollide3d;
extern crate nphysics3d;

use std::time::Duration;
use std::{hash::Hash, marker::PhantomData};

use amethyst::{
    assets::{Loader, ProgressCounter},
    controls::{FlyControlBundle, FlyControlTag},
    core::{
        nalgebra::{Quaternion, UnitQuaternion, Vector3},
        transform::TransformBundle,
        Transform,
    },
    ecs::{
        Component, Entities, Join, Read, ReadExpect, ReadStorage, System, VecStorage, Write,
        WriteExpect, WriteStorage,
    },
    input::InputBundle,
    input::InputHandler,
    prelude::*,
    renderer::{
        AmbientColor, Camera, DirectionalLight, DisplayConfig, DrawShaded, Light, Material,
        MaterialDefaults, MeshHandle, Pipeline, PosNormTex, Projection, RenderBundle, Rgba, Shape,
        Stage, VirtualKeyCode,
    },
    ui::{DrawUi, UiBundle, UiCreator},
    utils::application_root_dir,
};

use nphysics3d::{
    math::Vector,
    object::{BodyHandle, Material as PhysicsMaterial, Multibody, RigidBody},
    volumetric::Volumetric,
    world::World as PhysicsWorld,
};

use na::{Isometry3, Point3, Vector3 as PhysicsVector3};

use ncollide3d::{
    query::Ray,
    shape::{Cuboid, ShapeHandle},
    world::CollisionGroups,
};

pub struct PhysicsBody(BodyHandle);
type MyWorld = PhysicsWorld<f32>;

impl Component for PhysicsBody {
    type Storage = VecStorage<Self>;
}

#[derive(Default)]
struct ExampleState {
    cube_mesh: Option<MeshHandle>,
    cube_materials: Vec<Material>,
}

impl ExampleState {
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
        self.cube_mesh =
            Some(loader.load_from_data(mesh_data.into(), &mut progress, &mesh_storage));
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
        // *t.rotation_mut() = UnitQuaternion::new(0.5, 0.0, 0.5, 0.0);
        *t.translation_mut() = Vector3::new(
            (i as f32) * 3.0 - 7.5,
            0.5,
            3.0 + (-1.0_f32).powf(i as f32) * 0.5,
        );
        const COLLIDER_MARGIN: f32 = 0.01;

        let geom = ShapeHandle::new(Cuboid::new(PhysicsVector3::repeat(
            1000.5 - COLLIDER_MARGIN,
        )));
        let inertia = geom.inertia(1.0);
        let center_of_mass = geom.center_of_mass();

        let pos = {
            let translation = t.translation();
            Isometry3::new(
                PhysicsVector3::new(translation[0], translation[1], translation[2]),
                na::zero(),
            )
        };

        let body_handle = physics_world.add_rigid_body(pos, inertia, center_of_mass);

        let _collider_handle = physics_world.add_collider(
            COLLIDER_MARGIN,
            geom.clone(),
            body_handle,
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
            .build();
    }

    fn create_floor(&mut self, world: &mut World) {
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
            let plane: MeshHandle =
                loader.load_from_data(mesh_data.into(), &mut progress, &mesh_storage);
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

        world
            .create_entity()
            .named("floor")
            .with(plane)
            .with(color)
            .with(t)
            .build();
    }

    fn create_camera(&mut self, world: &mut World) {
        let mut t = Transform::default();
        *t.translation_mut() = Vector3::new(0.0, 1.8, 0.0);
        // *t.rotation_mut() = UnitQuaternion::from_quaternion(Quaternion::new(0.0, 0.0, 1.0, 0.0));
        *t.rotation_mut() = UnitQuaternion::new_observer_frame(
            &Vector3::new(0.0, 0.0, -1.0),
            &Vector3::new(0.0, 1.0, 0.0),
        );
        let c = Camera::from(Projection::perspective(1.3, 1.0471975512));
        world
            .create_entity()
            .named("camera")
            .with(c)
            .with(FlyControlTag)
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
pub struct RayMesh(MeshHandle);
pub struct RayMaterial(Material);

impl SimpleState for ExampleState {
    fn on_start(&mut self, data: StateData<GameData>) {
        data.world.register::<PhysicsBody>();
        let mut physics_world = MyWorld::default();
        physics_world.set_gravity(Vector::new(0., 0., 0.));
        self.create_light(data.world);
        self.create_floor(data.world);
        self.prepare_cubes(data.world);
        for i in 0..5 {
            self.create_cube(data.world, i, &mut physics_world);
        }
        self.create_camera(data.world);
        self.create_center(data.world);
        data.world.add_resource(physics_world);

        let (color, cylinder) = {
            let mesh_storage = data.world.read_resource();
            let tex_storage = data.world.read_resource();
            let mut progress = ProgressCounter::default();
            let loader = data.world.read_resource::<Loader>();
            let mesh_data = Shape::Cylinder(100, None).generate::<Vec<PosNormTex>>(None);
            let cylinder =
                RayMesh(loader.load_from_data(mesh_data.into(), &mut progress, &mesh_storage));
            let color = RayMaterial(Material {
                albedo: loader.load_from_data(
                    [220.0 / 255.0, 30.0 / 255.0, 23.0 / 255.0, 1.0].into(),
                    &mut progress,
                    &tex_storage,
                ),
                ..data.world.read_resource::<MaterialDefaults>().0.clone()
            });
            (color, cylinder)
        };
        data.world.add_resource(color);
        data.world.add_resource(cylinder);
    }
}

#[derive(Default)]
pub struct PointingSystem;

impl<'s> System<'s> for PointingSystem {
    type SystemData = (
        ReadStorage<'s, Camera>,
        Read<'s, MyWorld>,
        Entities<'s>,
        WriteStorage<'s, Transform>,
        ReadExpect<'s, RayMesh>,
        ReadExpect<'s, RayMaterial>,
        WriteStorage<'s, MeshHandle>,
        WriteStorage<'s, Material>,
        WriteExpect<'s, Loader>,
        Read<'s, InputHandler<String, String>>,
    );
    fn run(
        &mut self,
        (
            cameras,
            physics_world,
            entities,
            mut transforms,
            ray_mesh,
            ray_material,
            mut meshs,
            mut materials,
            loader,
            input,
        ): Self::SystemData,
    ) {
        let mut rotation = None;
        let mut translation = None;
        for (_, transform) in (&cameras, &transforms).join() {
            rotation = Some(transform.rotation().clone());
            translation = Some(transform.translation().clone());
        }
        let (rotation, translation) = match (rotation, translation) {
            (Some(r), Some(t)) => (r, t),
            (_, _) => return,
        };
        let r = rotation * Vector3::new(0.0, 0.0, 1.0);
        if input.keys_that_are_down().any(|k| k == VirtualKeyCode::Z) {
            let mut t = Transform::default();
            *t.translation_mut() = translation.clone();
            *t.scale_mut() = Vector3::new(0.02, 0.02, 5.0);
            *t.rotation_mut() =
                UnitQuaternion::new_observer_frame(&r, &Vector3::new(0.0, 1.0, 0.0));
            entities
                .build_entity()
                .with(t, &mut transforms)
                .with(ray_mesh.0.clone(), &mut meshs)
                .with(ray_material.0.clone(), &mut materials)
                .build();

            let ray = Ray::new(
                Point3::new(translation.x, translation.y, translation.z),
                PhysicsVector3::new(r.x, r.y, r.z),
            );
            let all_groups = &CollisionGroups::new();
            // println!("{:?}", rotation);
            println!("{:?}", ray);
            println!(
                "{:?}",
                physics_world
                    .collision_world()
                    .collision_objects()
                    .into_iter()
                    .map(|co| format!("{:?}", co.position()))
                    .collect::<Vec<_>>()
            );
            println!(
                "{}",
                physics_world
                    .collision_world()
                    .interferences_with_ray(&ray, all_groups)
                    .count()
            );
        }
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir();

    let display_config_path = format!("{}/resources/display_config.ron", app_root);

    let key_bindings_path = format!("{}/resources/input.ron", app_root);

    let pipe = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target([30.0 / 255.0, 144.0 / 255.0, 255.0 / 255.0, 1.0], 1.0)
            .with_pass(DrawShaded::<PosNormTex>::new())
            .with_pass(DrawUi::new()),
    );

    let game_data = GameDataBuilder::default()
        .with_bundle(
            FlyControlBundle::<String, String>::new(
                Some(String::from("move_x")),
                Some(String::from("move_y")),
                Some(String::from("move_z")),
            )
            .with_sensitivity(0.1, 0.1),
        )?
        .with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?,
        )?
        .with_bundle(TransformBundle::new().with_dep(&["fly_movement"]))?
        .with_bundle(UiBundle::<String, String>::new())?
        .with_bundle(
            RenderBundle::new(pipe, Some(DisplayConfig::load(&display_config_path)))
                .with_sprite_sheet_processor(),
        )?
        .with(
            PointingSystem::default(),
            "pointing_system",
            &["fly_movement"],
        );;
    let mut game = Application::new("./", ExampleState::default(), game_data)?;

    game.run();

    Ok(())
}
