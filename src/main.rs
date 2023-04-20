use bevy::prelude::*;

fn setup(mut commands: Commands) {
    commands.spawn(slime::species::SpeciesOptions {
        name: "first".to_owned(),
        num_agents: 10,
    });
    commands.spawn(Camera2dBundle::default());
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_plugin(slime::Plugin)
        .add_startup_system(setup)
        .run()
}
