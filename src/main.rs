extern crate amethyst;

use amethyst::{
    assets::{Loader, ProgressCounter},
    controls::{FlyControlBundle, FlyControlTag},
        input::InputBundle,

    core::{
        cgmath::{Quaternion, Rad, Vector3, Point3},
        transform::TransformBundle,
        Transform,
    },
    prelude::*,
    renderer::{
        AmbientColor, Camera, DrawShaded, Material, MaterialDefaults, ObjFormat, PosNormTex,
        Projection, Rgba, Shape, MeshHandle,
    },

    utils::application_root_dir,
};

struct ExampleState;

impl ExampleState {
    fn create_light(&mut self, world: &mut World) {
        world.add_resource(AmbientColor(Rgba(0.3, 0.3, 0.3, 1.0)));
    }

    fn create_cube(&mut self, world: &mut World) {
        let mut t = Transform::default();
        t.translation = Vector3::new(0.0, 3.0, 5.0);

        let (cube, red) = {
            let mesh_storage = world.read_resource();
            let tex_storage = world.read_resource();
            let mut progress = ProgressCounter::default();
            let loader = world.read_resource::<Loader>();
            let cube = loader.load(
                "resources/cube.obj",
                ObjFormat,
                (),
                &mut progress,
                &mesh_storage,
            );
            let red = Material {
                albedo: loader.load_from_data(
                    [0.0, 1.0, 0.0, 1.0].into(),
                    &mut progress,
                    &tex_storage,
                ),
                ..world.read_resource::<MaterialDefaults>().0.clone()
            };
            (cube, red)
        };

        world
            .create_entity()
            .named("box")
            .with(t)
            .with(cube)
            .with(red)
            .build();
    }

    fn create_floor(&mut self, world: &mut World) {
        let mut t = Transform::default();
        t.rotation = Quaternion::new(0.0, 1.0, 0.0, 0.0);
        t.scale = Vector3::new(1000.0, 1000.0, 1000.0);
        t.translation = Vector3::new(0.0, 0.0, 0.0);
        t.look_at(Point3::new(0.0, 1.0, 0.0), Vector3::new(0.0, 0.0, 1.0));

        let (plane, color) = {
            let mesh_storage = world.read_resource();
            let tex_storage = world.read_resource();
            let mut progress = ProgressCounter::default();
            let loader = world.read_resource::<Loader>();
            let mesh_data = Shape::Plane(None).generate::<Vec<PosNormTex>>(None);
            let plane: MeshHandle = loader.load_from_data(mesh_data.into(), &mut progress, &mesh_storage);
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

    fn create_wall(&mut self, world: &mut World) {
        let mut t = Transform::default();
        t.translation = Vector3::new(0.0, 0.0, 5.0);
        t.rotation = Quaternion::new(1.0, 0.0, 0.0, 0.0);
        t.scale = Vector3::new(5.0, 5.0, 0.0);
        t.look_at(Point3::new(0.0, 0.0, -1.0), Vector3::new(0.0, 1.0, 0.0));

        let (plane, color) = {
            let mesh_storage = world.read_resource();
            let tex_storage = world.read_resource();
            let mut progress = ProgressCounter::default();
            let loader = world.read_resource::<Loader>();
            let mesh_data = Shape::Plane(None).generate::<Vec<PosNormTex>>(None);
            let plane: MeshHandle = loader.load_from_data(mesh_data.into(), &mut progress, &mesh_storage);
            let color = Material {
                albedo: loader.load_from_data(
                    [1.0, 0.0, 1.0, 1.0].into(),
                    &mut progress,
                    &tex_storage,
                ),
                ..world.read_resource::<MaterialDefaults>().0.clone()
            };
            (plane, color)
        };

        world
            .create_entity()
            .named("wall")
            .with(plane)
            .with(color)
            .with(t)
            .build();
    }

    fn create_camera(&mut self, world: &mut World) {
        let mut t = Transform::default();
        t.translation = Vector3::new(0.0, 1.8, 0.0);
        t.rotation = Quaternion::new(0.0, 0.0, 1.0, 0.0);
        let c = Camera::from(Projection::perspective(1.3, Rad(1.0471975512)));
        world
            .create_entity()
            .named("camera")
            .with(c)
            .with(FlyControlTag)
            .with(t)
            .build();
    }
}

impl<'a, 'b> SimpleState<'a, 'b> for ExampleState {
    fn on_start(&mut self, data: StateData<GameData>) {
        self.create_light(data.world);
        self.create_floor(data.world);
        self.create_wall(data.world);
        self.create_cube(data.world);
        self.create_camera(data.world);
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());
let app_root = application_root_dir();
    let display_config_path = format!("{}/resources/display_config.ron", app_root);

    let key_bindings_path = format!("{}/resources/input.ron", app_root);

    let game_data = GameDataBuilder::default()
        .with_bundle(FlyControlBundle::<String, String>::new(
        Some(String::from("move_x")),
                Some(String::from("move_y")),
                Some(String::from("move_z"))).with_sensitivity(0.1, 0.1))?
                .with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?,
        )?
        .with_bundle(TransformBundle::new().with_dep(&["fly_movement"]))?
        .with_basic_renderer(display_config_path, DrawShaded::<PosNormTex>::new(), false)?;
    let mut game = Application::new("./", ExampleState, game_data)?;

    game.run();

    Ok(())
}
