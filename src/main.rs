//! A compute shader that simulates Conway's Game of Life.
//!
//! Compute shaders use the GPU for computing arbitrary information, that may be independent of what
//! is rendered to the screen.

use bevy::prelude::*;

fn setup(mut commands: Commands) {
    commands.spawn(slime::SpeciesOptions {
        name: "first".to_owned(),
        num_agents: 10,
    });
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_plugin(slime::Plugin)
        .add_startup_system(setup)
        .run()
}
