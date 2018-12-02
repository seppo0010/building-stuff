extern crate amethyst;

use amethyst::{
    assets::{Loader, ProgressCounter},
    controls::{ArcBallControlBundle, FlyControlTag},
    core::{
        cgmath::{Quaternion, Rad, Vector3},
        transform::TransformBundle,
        Transform,
    },
    prelude::*,
    renderer::{
        AmbientColor, Camera, DrawShaded, Material, MaterialDefaults, Mesh, ObjFormat, PosNormTex, PosTex,
        Projection, Rgba, Shape, MeshData,MeshHandle
    },
    utils::application_root_dir,
};

struct ExampleState;

impl ExampleState {
    fn create_light(&mut self, world: &mut World) {
        world.add_resource(AmbientColor(Rgba(0.1, 0.1, 0.1, 1.0)));
    }

    fn create_cube(&mut self, world: &mut World) {
        let mut t = Transform::default();
        t.translation = Vector3::new(0.0, 0.0, -5.0);

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
                    [1.0, 0.0, 0.0, 1.0].into(),
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
        t.translation = Vector3::new(5.0, 5.0, 5.0);
        t.rotation = Quaternion::new(0.0, 1.0, 0.0, 0.0);
        t.scale = Vector3::new(1000.0, 1000.0, 1000.0);

        let (plane, red) = {
            let mesh_storage = world.read_resource();
            let tex_storage = world.read_resource();
            let mut progress = ProgressCounter::default();
            let loader = world.read_resource::<Loader>();
            let mesh_data = Shape::Plane(None).generate::<Vec<PosNormTex>>(None);
            let v: Vec<PosNormTex> = match mesh_data {
                MeshData::PosNormTex(x) => x,
                _ => panic!("unexpected mesh data")
            };
            let plane: MeshHandle = loader.load_from_data(v.into(), &mut progress, &mesh_storage);
            let red = Material {
                albedo: loader.load_from_data(
                    [1.0, 0.0, 0.0, 1.0].into(),
                    &mut progress,
                    &tex_storage,
                ),
                ..world.read_resource::<MaterialDefaults>().0.clone()
            };
            (plane, red)
        };

        world
            .create_entity()
            .named("floor")
            .with(plane)
            .with(red)
            .with(t)
            .build();
    }

    fn create_camera(&mut self, world: &mut World) {
        let c = Camera::from(Projection::perspective(1.3, Rad(1.0471975512)));
        world
            .create_entity()
            .named("camera")
            .with(c)
            .with(FlyControlTag)
            .with(Transform::default())
            .build();
    }
}

impl<'a, 'b> SimpleState<'a, 'b> for ExampleState {
    fn on_start(&mut self, data: StateData<GameData>) {
        self.create_light(data.world);
        self.create_floor(data.world);
        self.create_camera(data.world);
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let display_config_path = format!("{}/resources/display_config.ron", application_root_dir());

    let game_data = GameDataBuilder::default()
        .with_bundle(ArcBallControlBundle::<String, String>::new().with_sensitivity(0.1, 0.1))?
        .with_bundle(TransformBundle::new().with_dep(&["free_rotation"]))?
        .with_basic_renderer(display_config_path, DrawShaded::<PosNormTex>::new(), false)?;
    let mut game = Application::new("./", ExampleState, game_data)?;

    game.run();

    Ok(())
}
