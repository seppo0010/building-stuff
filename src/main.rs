extern crate amethyst;

use amethyst::{
    assets::{Loader, ProgressCounter},
    controls::{FlyControlBundle, FlyControlTag},
    core::{
        cgmath::{Quaternion, Rad, Vector3},
        transform::TransformBundle,
        Transform,
    },
    input::InputBundle,
    prelude::*,
    renderer::{
        AmbientColor, Camera, DirectionalLight, DisplayConfig, DrawShaded, Light, Material,
        MaterialDefaults, MeshHandle, Pipeline, PosNormTex, Projection, RenderBundle, Rgba, Shape,
        Stage,
    },
    utils::application_root_dir,
};

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
            t.translation = Vector3::new(pos[0], pos[1], pos[2]);

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

    fn create_cube(&mut self, world: &mut World, i: usize) {
        let mut t = Transform::default();
        t.scale = Vector3::new(0.5, 0.5, 0.5);
        t.translation = Vector3::new(
            (i as f32) * 3.0 - 7.5,
            0.5,
            3.0 + (-1.0_f32).powf(i as f32) * 0.5,
        );
        world
            .create_entity()
            .named(format!("box{}", i))
            .with(t)
            .with(self.cube_mesh.clone().unwrap())
            .with(self.cube_materials[i].clone())
            .build();
    }

    fn create_floor(&mut self, world: &mut World) {
        let mut t = Transform::default();
        t.rotation = Quaternion::new(0.0, 1.0, 0.0, 0.0);
        t.scale = Vector3::new(1000.0, 0.0, 1000.0);
        t.translation = Vector3::new(0.0, 0.0, 0.0);

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
        self.prepare_cubes(data.world);
        for i in 0..5 {
            self.create_cube(data.world, i);
        }
        self.create_camera(data.world);
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
            .with_pass(DrawShaded::<PosNormTex>::new()),
    );

    let game_data = GameDataBuilder::default()
        .with_bundle(
            FlyControlBundle::<String, String>::new(
                Some(String::from("move_x")),
                Some(String::from("move_y")),
                Some(String::from("move_z")),
            ).with_sensitivity(0.1, 0.1),
        )?.with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?,
        )?.with_bundle(TransformBundle::new().with_dep(&["fly_movement"]))?
        .with_bundle(
            RenderBundle::new(pipe, Some(DisplayConfig::load(&display_config_path)))
                .with_sprite_sheet_processor(),
        )?;
    let mut game = Application::new("./", ExampleState::default(), game_data)?;

    game.run();

    Ok(())
}
