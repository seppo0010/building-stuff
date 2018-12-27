extern crate amethyst;
extern crate nalgebra as na;
extern crate ncollide3d;
extern crate nphysics3d;
// extern crate nphysics_testbed3d;
extern crate specs;

use std::{
    cmp::Ordering,
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
    input::{InputBundle, InputHandler},
    prelude::*,
    renderer::{
        AmbientColor, Camera, DirectionalLight, DisplayConfig, DrawShaded, Light, Material,
        MaterialDefaults, MeshHandle, MouseButton, Pipeline, PosNormTex, Projection, RenderBundle,
        Rgba, Shape, Stage,
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
    algebra::Velocity3,
    object::{BodyHandle, Material as PhysicsMaterial},
    volumetric::Volumetric,
    world::World as PhysicsWorld,
};
use specs::{Entities, Entity};

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

pub struct PhysicsBody(CollisionObjectHandle);
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
        for i in 0..5 {
            self.create_cube(data.world, i, &mut physics_world);
        }
        physics_world.step();
        physics_world.set_gravity(-PhysicsVector3::y() * 9.81);
        self.create_camera(data.world);
        self.create_center(data.world);

        // let mut testbed = nphysics_testbed3d::Testbed::new(physics_world.inner);
        // testbed.look_at(Point3::new(-4.0, 1.0, -4.0), Point3::new(0.0, 1.0, 0.0));
        // testbed.run();

        data.world.add_resource(physics_world);
    }
}

struct SelectedObject {
    entity: Entity,
    previous_camera_position: Isometry3<f32>,
}

#[derive(Default)]
pub struct PointingSystem {
    selected_object: Option<SelectedObject>,
}

impl PointingSystem {
    fn find_current_ray(
        &self,
        cameras: &ReadStorage<Camera>,
        transforms: &ReadStorage<Transform>,
    ) -> (Ray<f32>, Isometry3<f32>) {
        let isometry = (cameras, transforms).join().next().unwrap().1.isometry();
        let r = isometry.rotation * Vector3::new(0.0, 0.0, 1.0);
        (
            Ray::new(
                Point3::new(
                    isometry.translation.vector.x,
                    isometry.translation.vector.y,
                    isometry.translation.vector.z,
                ),
                PhysicsVector3::new(-r.x, -r.y, -r.z),
            ),
            *isometry,
        )
    }

    fn find_pointed_object(
        &self,
        ray: &Ray<f32>,
        entities: &Entities,
        physics_world: &Write<MyWorld>,
        physics_bodies: &WriteStorage<PhysicsBody>,
    ) -> Option<Entity> {
        let all_groups = &CollisionGroups::new();

        let handle = physics_world
            .collision_world()
            .interferences_with_ray(&ray, all_groups)
            .into_iter()
            .min_by(|(_, inter1), (_, inter2)| {
                inter1
                    .toi
                    .partial_cmp(&inter2.toi)
                    .unwrap_or(Ordering::Equal)
            })
            .map(|(col, _)| col.handle());

        (entities, physics_bodies)
            .join()
            .filter(|(_, b)| Some(b.0) == handle)
            .next()
            .map(|(e, _)| e)
    }

    fn move_selected_object(
        &mut self,
        cameras: &ReadStorage<Camera>,
        transforms: &ReadStorage<Transform>,
        physics_bodies: &WriteStorage<PhysicsBody>,
        world: &mut Write<MyWorld>,
    ) {
        let camera_isometry = self.find_current_ray(cameras, transforms).1;
        let so = match self.selected_object.as_mut() {
            Some(x) => x,
            None => return,
        };
        let body = match physics_bodies.get(so.entity) {
            Some(x) => x,
            None => return,
        };
        let linear =
            camera_isometry.translation.vector - so.previous_camera_position.translation.vector;
        let bh = match world.collider_body_handle(body.0) {
            Some(x) => x,
            None => return,
        };
        let rb = world.rigid_body_mut(bh).unwrap();
        rb.apply_displacement(&Velocity3::new(linear, Vector3::new(0.0, 0.0, 0.0)));
        so.previous_camera_position = camera_isometry;
    }

    fn grab_object(
        &mut self,
        entities: &Entities,
        cameras: &ReadStorage<Camera>,
        physics_world: &Write<MyWorld>,
        transforms: &ReadStorage<Transform>,
        physics_bodies: &WriteStorage<PhysicsBody>,
    ) {
        let (ray, camera_isometry) = self.find_current_ray(&cameras, &transforms);

        self.selected_object = self
            .find_pointed_object(&ray, entities, &physics_world, physics_bodies)
            .map(|entity| SelectedObject {
                entity: entity,
                previous_camera_position: camera_isometry,
            });
    }

    fn drop_object(&mut self) {
        self.selected_object = None;
    }
}

impl<'s> System<'s> for PointingSystem {
    type SystemData = (
        Entities<'s>,
        ReadStorage<'s, Camera>,
        Write<'s, MyWorld>,
        ReadStorage<'s, Transform>,
        WriteStorage<'s, PhysicsBody>,
        Read<'s, InputHandler<String, String>>,
    );
    fn run(
        &mut self,
        (entities, cameras, mut physics_world, transforms, physics_bodies, input): Self::SystemData,
    ) {
        let is_left_click = input.mouse_button_is_down(MouseButton::Left);
        match (is_left_click, self.selected_object.is_some()) {
            (true, true) => self.move_selected_object(
                &cameras,
                &transforms,
                &physics_bodies,
                &mut physics_world,
            ),
            (true, false) => self.grab_object(
                &entities,
                &cameras,
                &physics_world,
                &transforms,
                &physics_bodies,
            ),
            (false, true) => self.drop_object(),
            (false, false) => (),
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
