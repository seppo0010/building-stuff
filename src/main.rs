extern crate amethyst;
extern crate nalgebra as na;
extern crate ncollide3d;
extern crate nphysics3d;
// extern crate nphysics_testbed3d;
extern crate specs;

use std::{
    cmp::Ordering,
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use amethyst::{
    assets::{Loader, ProgressCounter},
    controls::{FlyControlBundle, FlyControlTag},
    core::{
        nalgebra::{UnitQuaternion, Vector3},
        transform::TransformBundle,
        Transform,
    },
    ecs::{Component, Join, Read, ReadStorage, System, VecStorage, Write, WriteStorage},
    input::InputBundle,
    prelude::*,
    renderer::{
        AmbientColor, Camera, DirectionalLight, DisplayConfig, DrawShaded, Light, Material,
        MaterialDefaults, MeshHandle, Pipeline, PosNormTex, Projection, RenderBundle, Rgba, Shape,
        Stage,
    },
    ui::{DrawUi, UiBundle, UiCreator},
    utils::application_root_dir,
};

use na::{Isometry3, Point3, Vector3 as PhysicsVector3};

use ncollide3d::{
    query::Ray,
    shape::{Cuboid, ShapeHandle},
    world::{CollisionGroups, CollisionObjectHandle},
};
use nphysics3d::{
    object::{BodyHandle, ColliderHandle, Material as PhysicsMaterial},
    volumetric::Volumetric,
    world::World as PhysicsWorld,
};

const COLLIDER_MARGIN: f32 = 0.01;

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

#[derive(Clone, PartialEq)]
pub struct CubeName(String);
impl Component for CubeName {
    type Storage = VecStorage<Self>;
}

pub struct PhysicsBody(CollisionObjectHandle);
impl Component for PhysicsBody {
    type Storage = VecStorage<Self>;
}

#[derive(Default)]
struct ExampleState {
    cube_mesh: Option<MeshHandle>,
    cube_materials: Vec<Material>,
    cube_names: Vec<String>,
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
        for (color, name) in [
            ([0.0, 1.0, 0.0, 1.0], "green"),
            ([1.0, 1.0, 0.0, 1.0], "yellow"),
            ([1.0, 0.0, 0.0, 1.0], "red"),
            ([1.0, 0.0, 1.0, 1.0], "pink"),
            ([0.0, 0.0, 1.0, 1.0], "blue"),
        ]
        .into_iter()
        {
            self.cube_names.push(name.to_string());
            self.cube_materials.push(Material {
                albedo: loader.load_from_data((*color).into(), &mut progress, &tex_storage),
                ..world.read_resource::<MaterialDefaults>().0.clone()
            });
        }
    }

    fn create_cube(
        &mut self,
        world: &mut World,
        i: usize,
        physics_world: &mut MyWorld,
        cube_names: &mut HashMap<ColliderHandle, CubeName>,
    ) {
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
            Vector3::new(0.9, 0.1, 0.0)
        );
        let handle = physics_world.add_rigid_body(pos, inertia, center_of_mass);

        let body_handle = physics_world.add_collider(
            COLLIDER_MARGIN,
            geom.clone(),
            handle,
            Isometry3::identity(),
            PhysicsMaterial::default(),
        );
        cube_names.insert(body_handle, CubeName(self.cube_names[i].clone()));

        world
            .create_entity()
            .named(format!("box{}", i))
            .with(t)
            .with(self.cube_mesh.clone().unwrap())
            .with(self.cube_materials[i].clone())
            .with(PhysicsBody(body_handle))
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

    fn create_camera(&mut self, world: &mut World) {
        let mut t = Transform::default();
        *t.translation_mut() = Vector3::new(0.0, 1.8, 0.0);
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
impl SimpleState for ExampleState {
    fn on_start(&mut self, data: StateData<GameData>) {
        data.world.register::<PhysicsBody>();
        let mut physics_world = MyWorld::default();
        self.create_light(data.world);
        self.create_floor(data.world, &mut physics_world);
        self.prepare_cubes(data.world);
        let mut cube_names = HashMap::with_capacity(5);
        for i in 0..5 {
            self.create_cube(data.world, i, &mut physics_world, &mut cube_names);
        }
        physics_world.step();
        physics_world.set_gravity(-PhysicsVector3::y() * 9.81);
        self.create_camera(data.world);
        self.create_center(data.world);

        // let mut testbed = nphysics_testbed3d::Testbed::new(physics_world.inner);
        // testbed.look_at(Point3::new(-4.0, 1.0, -4.0), Point3::new(0.0, 1.0, 0.0));
        // testbed.run();

        data.world.add_resource(physics_world);
        data.world.add_resource(cube_names);
    }
}

#[derive(Default)]
pub struct PointingSystem {
    pointed_cube: Option<CubeName>,
}

impl<'s> System<'s> for PointingSystem {
    type SystemData = (
        ReadStorage<'s, Camera>,
        Read<'s, MyWorld>,
        ReadStorage<'s, Transform>,
        Read<'s, HashMap<ColliderHandle, CubeName>>,
    );
    fn run(
        &mut self,
        (cameras, physics_world, transforms, cube_names): Self::SystemData,
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
        let ray = Ray::new(
            Point3::new(translation.x, translation.y, translation.z),
            PhysicsVector3::new(-r.x, -r.y, -r.z),
        );
        let all_groups = &CollisionGroups::new();
        let current_cube_name = physics_world
            .collision_world()
            .interferences_with_ray(&ray, all_groups)
            .into_iter()
            .min_by(|(_, inter1), (_, inter2)| {
                inter1
                    .toi
                    .partial_cmp(&inter2.toi)
                    .unwrap_or(Ordering::Equal)
            })
            .and_then(|(col, _)| cube_names.get(&col.handle()));

        if current_cube_name != self.pointed_cube.as_ref() {
            self.pointed_cube = current_cube_name.map(|x| x.clone());
            println!(
                "watching {}",
                current_cube_name.map(|x| &*x.0).unwrap_or("no cube")
            );
        }
    }
}

#[derive(Default)]
pub struct PhysicsSystem;

impl<'s> System<'s> for PhysicsSystem {
    type SystemData = (
        Write<'s, MyWorld>,
        WriteStorage<'s, Transform>,
        ReadStorage<'s, PhysicsBody>,
    );
    fn run(&mut self, (mut physics_world, mut transforms, bodies): Self::SystemData) {
        physics_world.step();
        for (mut t, body) in (&mut transforms, &bodies).join() {
            if let Some(pos) = physics_world
                .collision_world()
                .collision_object(body.0)
                .map(|co| co.position())
            {
                *t.translation_mut() = pos.translation.vector;
                *t.rotation_mut() = pos.rotation;
            }
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
        )
        .with(
            PhysicsSystem::default(),
            "physics_system",
            &["pointing_system"],
        );
    let mut game = Application::new("./", ExampleState::default(), game_data)?;

    game.run();

    Ok(())
}
