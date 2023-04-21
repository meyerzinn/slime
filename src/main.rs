use bevy::prelude::*;

fn setup(mut commands: Commands) {
    commands.spawn(slime::species::SpeciesOptions {
        name: "first".to_owned(),
        num_agents: 10000,
        color: Color::RED,
        speed: 5e-6,
    });
    commands.spawn(slime::species::SpeciesOptions {
        name: "second".to_owned(),
        num_agents: 10000,
        color: Color::YELLOW,
        speed: 1e-6,
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
