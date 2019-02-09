extern crate amethyst;
extern crate ncollide3d;
extern crate nphysics3d;
#[cfg(feature = "testbed")]
extern crate nphysics_testbed3d;
extern crate specs;
extern crate winit;

mod components;
mod game_state;
mod resources;
mod systems;
mod utils;

use crate::{
    game_state::GameState,
    systems::{MergeSystem, MovingSystem, PhysicsSystem, RotationSystem, TranslationSystem},
};

use amethyst::{
    controls::{CursorHideSystem, MouseFocusUpdateSystem},
    core::transform::TransformBundle,
    input::InputBundle,
    prelude::*,
    renderer::{DisplayConfig, DrawShaded, Pipeline, PosNormTex, RenderBundle, Stage},
    ui::{DrawUi, UiBundle},
    utils::application_root_dir,
};

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root_path = application_root_dir().unwrap();
    let app_root = app_root_path.to_str().unwrap();

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
        .with(RotationSystem::default(), "rotation_system", &[])
        .with(TranslationSystem::default(), "translation_system", &[])
        .with_bundle(TransformBundle::new().with_dep(&[]))?
        .with_bundle(UiBundle::<String, String>::new())?
        .with_bundle(
            RenderBundle::new(pipe, Some(DisplayConfig::load(&display_config_path)))
                .with_sprite_sheet_processor(),
        )?
        .with(MouseFocusUpdateSystem::new(), "mouse_focus", &[])
        .with(CursorHideSystem::new(), "cursor_hide", &["mouse_focus"])
        .with(
            MergeSystem::default(),
            "merge_system",
            &["rotation_system", "translation_system"],
        )
        .with(MovingSystem::default(), "move_system", &["merge_system"])
        .with(PhysicsSystem::default(), "physics_system", &["move_system"]);
    let mut game = Application::new("./", GameState::default(), game_data)?;

    game.run();

    Ok(())
}
