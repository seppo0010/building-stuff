extern crate amethyst;

use amethyst::{
    assets::{Loader, ProgressCounter},
    controls::{ArcBallControlBundle, FlyControlTag},
    core::cgmath::Rad,
    core::transform::TransformBundle,
    core::{cgmath::Vector3, Transform},
    prelude::*,
    renderer::{AmbientColor, Camera, Material, MaterialDefaults, ObjFormat, Projection, Rgba},
    renderer::{DrawShaded, PosNormTex},
    utils::application_root_dir,
};

struct ExampleState;

impl<'a, 'b> SimpleState<'a, 'b> for ExampleState {
    fn on_start(&mut self, data: StateData<GameData>) {
        data.world
            .add_resource(AmbientColor(Rgba(0.1, 0.1, 0.1, 1.0)));

        let mut t = Transform::default();
        t.translation = Vector3::new(0.0, 0.0, -5.0);

        let (cube, red) = {
            let loader = data.world.read_resource::<Loader>();
            let mut progress = ProgressCounter::default();
            let mesh_storage = data.world.read_resource();
            let tex_storage = data.world.read_resource();
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
                ..data.world.read_resource::<MaterialDefaults>().0.clone()
            };
            (cube, red)
        };

        data.world
            .create_entity()
            .named("box")
            .with(t)
            .with(cube)
            .with(red)
            .build();

        let c = Camera::from(Projection::perspective(1.3, Rad(1.0471975512)));
        data.world
            .create_entity()
            .named("camera")
            .with(c)
            .with(FlyControlTag)
            .with(Transform::default())
            .build();
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
