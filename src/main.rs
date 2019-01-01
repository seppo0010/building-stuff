extern crate amethyst;
extern crate nalgebra as na;
extern crate ncollide3d;
extern crate nphysics3d;
extern crate nphysics_testbed3d;
extern crate specs;
extern crate winit;

use std::{
    cmp::Ordering,
    f32,
    ops::{Deref, DerefMut},
};

use amethyst::{
    assets::{Loader, ProgressCounter},
    controls::{CursorHideSystem, HideCursor, MouseFocusUpdateSystem, WindowFocus},
    core::{
        nalgebra::{Unit, UnitQuaternion, Vector3},
        timing::Time,
        transform::TransformBundle,
        Transform,
    },
    ecs::{Component, Join, Read, ReadStorage, System, VecStorage, FlaggedStorage, Write, WriteStorage},
    input::{get_input_axis_simple, InputBundle, InputHandler},
    prelude::*,
    renderer::{
        AmbientColor, Camera, DirectionalLight, DisplayConfig, DrawShaded, Light, Material,
        MaterialDefaults, MeshHandle, MouseButton, Pipeline, PosNormTex, Projection, RenderBundle,
        Rgba, Shape, Stage,
    },
    shrev::{EventChannel, ReaderId},
    ui::{DrawUi, UiBundle, UiCreator},
    utils::application_root_dir,
};

use na::{Isometry3, Point3, Vector3 as PhysicsVector3};

use ncollide3d::{
    bounding_volume::{AABB, HasBoundingVolume},
    query::Ray,
    shape::{Cuboid, Cylinder, ShapeHandle, TriMesh},
    transformation::ToTriMesh,
    world::{CollisionGroups, CollisionObjectHandle},
};
use nphysics3d::{
    algebra::Inertia3,
    force_generator::{ConstantAcceleration, ForceGeneratorHandle},
    object::{BodyHandle, Material as PhysicsMaterial, RigidBody},
    volumetric::Volumetric,
    world::World as PhysicsWorld,
};
use specs::{prelude::Resources, Entities, Entity};
use winit::{DeviceEvent, Event};

const COLLIDER_MARGIN: f32 = 0.01;
const MAGIC_LINEAR_SPEED_MULTIPLIER: f32 = 60.0;
const MAGIC_ANGULAR_VELOCITY_MULTIPLIER: f32 = 50.0;

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

pub struct CameraSelf;
impl Component for CameraSelf {
    type Storage = FlaggedStorage<Self>;
}

#[derive(Default)]
struct GameState {
    cube_mesh: Option<MeshHandle>,
    cube_materials: Vec<Material>,
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
        let cylinder = Cylinder::new(0.9, 0.25);
        let mut t = cylinder.to_trimesh(10);
        t.unify_index_buffer();
        let aabb: AABB<f32> = cylinder.bounding_volume(&Isometry3::identity());
        let geom = ShapeHandle::new(TriMesh::new(t.coords, t.indices.unwrap_unified().into_iter().map(|p| Point3::new(p.coords.x as usize, p.coords.y as usize, p.coords.z as usize)).collect(), t.uvs));
        let inertia = Inertia3::zero();
        let center_of_mass = aabb.center();

        let pos = Isometry3::new(
            PhysicsVector3::new(0.0, 3.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
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

        let mut testbed = nphysics_testbed3d::Testbed::new(physics_world.inner);
        testbed.look_at(Point3::new(-4.0, 1.0, -4.0), Point3::new(0.0, 1.0, 0.0));
        testbed.run();

        // data.world.add_resource(physics_world);
    }
}

struct SelectedObject {
    entity: Entity,
    previous_camera_position: Isometry3<f32>,
    force: ForceGeneratorHandle,
    distance: f32,
    box_forward: Vector3<f32>,
    box_up: Vector3<f32>,
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
    ) -> Option<(Entity, f32)> {
        let all_groups = &CollisionGroups::new();

        let handle = physics_world
            .collision_world()
            .interferences_with_ray(&ray, all_groups)
            .min_by(|(_, inter1), (_, inter2)| {
                inter1
                    .toi
                    .partial_cmp(&inter2.toi)
                    .unwrap_or(Ordering::Equal)
            })
            .map(|(col, inter)| (col.handle(), inter.toi));

        (entities, physics_bodies)
            .join()
            .find(|(_, b)| Some(b.0) == handle.map(|x| x.0))
            .map(|(e, _)| (e, handle.unwrap().1))
    }

    fn get_selected_object_rigid_body_mut<'a>(
        &mut self,
        physics_bodies: &WriteStorage<PhysicsBody>,
        world: &'a mut Write<MyWorld>,
    ) -> Option<&'a mut RigidBody<f32>> {
        self.selected_object
            .as_mut()
            .and_then(|so| physics_bodies.get(so.entity))
            .and_then(|body| world.collider_body_handle(body.0))
            .and_then(move |bh| world.rigid_body_mut(bh))
    }

    fn move_selected_object(
        &mut self,
        cameras: &ReadStorage<Camera>,
        transforms: &ReadStorage<Transform>,
        physics_bodies: &WriteStorage<PhysicsBody>,
        world: &mut Write<MyWorld>,
    ) {
        let camera_isometry = self.find_current_ray(cameras, transforms).1;
        let rb = match self.get_selected_object_rigid_body_mut(physics_bodies, world) {
            Some(x) => x,
            None => return,
        };
        let so = self.selected_object.as_mut().unwrap();
        let linear = camera_isometry.translation.vector
            - so.previous_camera_position.translation.vector
            + (so.previous_camera_position.rotation * Vector3::new(0.0, 0.0, 1.0)
                - camera_isometry.rotation * Vector3::new(0.0, 0.0, 1.0))
                * so.distance;
        let angular = (rb.position().rotation * so.box_forward)
            .cross(&(camera_isometry.rotation * Vector3::z()))
            + (rb.position().rotation * so.box_up)
                .cross(&(camera_isometry.rotation * Vector3::y()));
        rb.set_linear_velocity(linear * MAGIC_LINEAR_SPEED_MULTIPLIER);
        rb.set_angular_velocity(angular * MAGIC_ANGULAR_VELOCITY_MULTIPLIER);
        so.previous_camera_position = camera_isometry;
    }

    fn grab_object(
        &mut self,
        entities: &Entities,
        cameras: &ReadStorage<Camera>,
        physics_world: &mut Write<MyWorld>,
        transforms: &ReadStorage<Transform>,
        physics_bodies: &WriteStorage<PhysicsBody>,
    ) {
        let (ray, camera_isometry) = self.find_current_ray(&cameras, &transforms);

        self.selected_object = self
            .find_pointed_object(&ray, entities, &physics_world, physics_bodies)
            .filter(|(_, toi)| *toi < 4.0)
            .map(|(entity, toi)| {
                let mut f = ConstantAcceleration::new(
                    -physics_world.gravity(),
                    Vector3::new(0.0, 0.0, 0.0),
                );
                // this is awful
                f.add_body_part(
                    physics_world
                        .collider_body_handle(physics_bodies.get(entity).unwrap().0)
                        .unwrap(),
                );
                (entity, physics_world.add_force_generator(f), toi)
            })
            .map(|(entity, antig, toi)| {
                let rot_inv = physics_world
                    .rigid_body(
                        physics_world
                            .collider_body_handle(physics_bodies.get(entity).unwrap().0)
                            .unwrap(),
                    )
                    .unwrap()
                    .position()
                    .rotation
                    .inverse();
                SelectedObject {
                    entity,
                    previous_camera_position: camera_isometry,
                    force: antig,
                    distance: toi,
                    box_forward: rot_inv * (camera_isometry.rotation * Vector3::z()),
                    box_up: rot_inv * (camera_isometry.rotation * Vector3::y()),
                }
            });
    }

    fn drop_object(&mut self, physics_world: &mut Write<MyWorld>) {
        if let Some(ref so) = self.selected_object {
            physics_world.remove_force_generator(so.force);
        }
        self.selected_object = None;
    }
}

type PointingSystemData<'s> = (
    Entities<'s>,
    ReadStorage<'s, Camera>,
    Write<'s, MyWorld>,
    ReadStorage<'s, Transform>,
    WriteStorage<'s, PhysicsBody>,
    Read<'s, InputHandler<String, String>>,
);
impl<'s> System<'s> for PointingSystem {
    type SystemData = PointingSystemData<'s>;
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
                &mut physics_world,
                &transforms,
                &physics_bodies,
            ),
            (false, true) => self.drop_object(&mut physics_world),
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
                .collider_body_handle(body.0)
                .and_then(|bh| physics_world.rigid_body(bh))
                .map(|co| co.position())
            {
                *t.translation_mut() = pos.translation.vector;
                *t.rotation_mut() = pos.rotation;
            }
        }
    }
}

pub struct MovementSystem {
    sensitivity_x: f32,
    sensitivity_y: f32,
    speed: f32,
    event_reader: Option<ReaderId<Event>>,
}

impl Default for MovementSystem {
    fn default() -> Self {
        MovementSystem {
            sensitivity_x: 0.2,
            sensitivity_y: 0.2,
            speed: 1.0,
            event_reader: None,
        }
    }
}

impl<'s> System<'s> for MovementSystem {
    type SystemData = (
        Read<'s, EventChannel<Event>>,
        WriteStorage<'s, Transform>,
        ReadStorage<'s, Camera>,
        Read<'s, WindowFocus>,
        Read<'s, HideCursor>,
        Read<'s, Time>,
        Read<'s, InputHandler<String, String>>,
    );

    fn run(
        &mut self,
        (events, mut transforms, cameras, focus, hide, time, input): Self::SystemData,
    ) {
        if focus.is_focused && hide.hide {
            let x = get_input_axis_simple(&Some("move_x".to_owned()), &input);
            let z = get_input_axis_simple(&Some("move_z".to_owned()), &input);
            if let Some(dir) = Unit::try_new(Vector3::new(x, 0.0, z), 1.0e-6) {
                for (transform, _) in (&mut transforms, &cameras).join() {
                    let mut iso = transform.isometry_mut();
                    let d = iso.rotation * dir.as_ref();
                    let total = d.x.abs() + d.z.abs();
                    iso.translation.vector += Vector3::new(d.x / total, 0.0, d.z / total)
                        * time.delta_seconds()
                        * self.speed;
                }
            }
            for event in events.read(
                &mut self
                    .event_reader
                    .as_mut()
                    .expect("`MovementSystem::setup` was not called before `MovementSystem::run`"),
            ) {
                if let Event::DeviceEvent { ref event, .. } = *event {
                    if let DeviceEvent::MouseMotion { delta: (x, y) } = *event {
                        for (transform, _) in (&mut transforms, &cameras).join() {
                            transform.pitch_local((-y as f32 * self.sensitivity_y).to_radians());
                            transform.yaw_global((-x as f32 * self.sensitivity_x).to_radians());
                            // there's probably a better way to do this if you know trigonometry :see_no_evil:
                            while (transform.isometry().rotation * Vector3::z()).y < -0.8 {
                                transform.pitch_local((-1.0_f32).to_radians());
                            }
                            while (transform.isometry().rotation * Vector3::z()).y > 0.8 {
                                transform.pitch_local((1.0_f32).to_radians());
                            }
                        }
                    }
                }
            }
        }
    }

    fn setup(&mut self, res: &mut Resources) {
        use amethyst::core::specs::prelude::SystemData;

        Self::SystemData::setup(res);
        self.event_reader = Some(res.fetch_mut::<EventChannel<Event>>().register_reader());
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir();

    let display_config_path = format!("{}/resources/display_config.ron", app_root);

    let key_bindings_path = format!("{}/resources/input.ron", app_root);

    let pipe = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target([30.0 / 255.0, 144.0 / 255.0, 1.0, 1.0], 1.0)
            .with_pass(DrawShaded::<PosNormTex>::new())
            .with_pass(DrawUi::new()),
    );

    let game_data = GameDataBuilder::default()
        .with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?,
        )?
        .with(MovementSystem::default(), "movement_system", &[])
        .with_bundle(TransformBundle::new().with_dep(&["movement_system"]))?
        .with_bundle(UiBundle::<String, String>::new())?
        .with_bundle(
            RenderBundle::new(pipe, Some(DisplayConfig::load(&display_config_path)))
                .with_sprite_sheet_processor(),
        )?
        .with(
            MouseFocusUpdateSystem::new(),
            "mouse_focus",
            &["movement_system"],
        )
        .with(CursorHideSystem::new(), "cursor_hide", &["mouse_focus"])
        .with(
            PointingSystem::default(),
            "pointing_system",
            &["movement_system"],
        )
        .with(
            PhysicsSystem::default(),
            "physics_system",
            &["pointing_system"],
        );
    let mut game = Application::new("./", GameState::default(), game_data)?;

    game.run();

    Ok(())
}
