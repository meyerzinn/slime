use std::{
    f32::consts::PI,
    ops::{Add, RangeInclusive},
};

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
    winit::WinitSettings,
};
use bevy_egui::{
    egui::{self, TextBuffer},
    EguiContexts, EguiPlugin, EguiSet,
};
use slime::{
    species::{NumAgents, Qualities},
    Options,
};

const EVAPORATION_DELTA: f32 = 1e-4;
const DIFFUSION_DELTA: f32 = 1e-4;
const SPEED_DELTA: f32 = 1e-7;
const TURN_SPEED_DELTA: f32 = 1e-5;
const VIEW_DISTANCE_DELTA: f32 = 1e-4;
const FIELD_OF_VIEW_DELTA: f32 = 1e-3;

fn setup(mut commands: Commands, mut windows: Query<&mut Window>) {
    windows.single_mut().title = "slime by Meyer Zinn".to_owned();

    commands.insert_resource(slime::Options {
        evaporation: 0.001,
        diffusion: 1.0,
    });

    // commands.spawn(slime::SpeciesBundle {
    //     name: "first".to_owned().into(),
    //     count: 10000.into(),
    //     color: Color::WHITE.into(),
    //     speed: (1e-5).into(),
    //     turn_speed: (0.0005).into(),
    //     view_distance: (0.005).into(),
    //     ..default()
    // });

    // commands.spawn(slime::SpeciesBundle {
    //     name: "second".to_owned().into(),
    //     count: 5000.into(),
    //     color: Color::RED.into(),
    //     speed: (1e-5).into(),
    //     turn_speed: (0.00025).into(),
    //     view_distance: (0.005).into(),
    //     ..default()
    // });

    // commands.spawn(slime::SpeciesBundle {
    //     name: "first".to_owned().into(),
    //     count: 10000.into(),
    //     color: Color::RED.into(),
    //     speed: (5e-6).into(),
    //     ..default()
    // });
    // commands.spawn(slime::species::SpeciesBundle {
    //     name: "second".to_owned().into(),
    //     count: 5000.into(),
    //     color: Color::YELLOW.into(),
    //     speed: (1e-6).into(),
    //     view_distance: 1e-3.into(),
    //     ..default()
    // });
    // commands.spawn(slime::species::SpeciesBundle {
    //     name: "fast".to_owned().into(),
    //     count: 5000.into(),
    //     color: Color::GREEN.into(),
    //     speed: (1e-5).into(),
    //     view_distance: 1e-3.into(),
    //     ..default()
    // });

    commands.spawn(Camera2dBundle::default());
}

#[derive(Resource)]
struct UiState {
    selected: Option<Entity>,
    vsync: bool,
    new_species_name: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            selected: Default::default(),
            vsync: true,
            new_species_name: Default::default(),
        }
    }
}

fn draw_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    diagnostics: Res<Diagnostics>,
    mut options: ResMut<Options>,
    species_query: Query<(Entity, &Name, &mut NumAgents, &mut Qualities)>,
) {
    let fps = diagnostics
        .get_measurement(FrameTimeDiagnosticsPlugin::FPS)
        .map(|x| x.value)
        .unwrap_or(0.);
    egui::Window::new("Slime").show(contexts.ctx_mut(), |ui| {
        {
            ui.heading("Statistics");

            ui.label(format!("FPS: {}", fps.round()));
            let count = species_query.iter().count();
            ui.label(format!("Species: {}", count));
            let agents = species_query
                .iter()
                .map(|(_, _, count, _)| **count)
                .fold(0, Add::add);
            ui.label(format!("Agents: {}", agents));
        }
        ui.separator();

        ui.heading("Simulation");
        ui.checkbox(&mut ui_state.vsync, "VSync");

        let Options {
            mut evaporation,
            mut diffusion,
        } = options.clone();
        let mut options_changed = false;
        options_changed |= ui
            .horizontal(|ui| {
                let ret = ui
                    .add(egui::DragValue::new(&mut evaporation).speed(EVAPORATION_DELTA))
                    .changed();
                ui.label("Evaporation");
                ret
            })
            .inner;

        options_changed |= ui
            .horizontal(|ui| {
                let ret = ui
                    .add(egui::DragValue::new(&mut diffusion).speed(DIFFUSION_DELTA))
                    .changed();
                ui.label("Diffusion");
                ret
            })
            .inner;

        if options_changed {
            *options = Options {
                evaporation: evaporation.clamp(0.0, 1.0),
                diffusion: diffusion.clamp(0.0, 1.0),
            };
        }

        ui.separator();

        let create = ui
            .horizontal(|ui| {
                let name_entry = ui.text_edit_singleline(&mut ui_state.new_species_name);
                let create_button = ui.add_enabled(
                    !ui_state.new_species_name.is_empty(),
                    egui::Button::new("Create"),
                );
                (name_entry.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    || create_button.clicked()
            })
            .inner;

        if create {
            let mut ent = commands.spawn((
                Name::new(ui_state.new_species_name.take()),
                slime::SpeciesBundle {
                    num_agents: NumAgents::from(50000),
                    qualities: Qualities::default(),
                },
            ));
            ent.log_components();
            ui_state.selected = Some(ent.id());
        }

        // which species is selected?
        ui_state.selected = ui_state
            .selected
            .and_then(|id| species_query.get(id).ok().map(|(id, _, _, _)| id));

        ui.horizontal(|ui| {
            // logic for the combobox
            egui::ComboBox::from_label("Species")
                .selected_text(
                    ui_state
                        .selected
                        .map(|id| {
                            format!(
                                "[{:?}] {}",
                                id,
                                species_query.get_component::<Name>(id).unwrap(),
                            )
                        })
                        .unwrap_or("[no species]".to_owned()),
                )
                .show_ui(ui, |ui| {
                    if species_query.is_empty() {
                        ui.set_enabled(false);
                    }
                    for (id, name, _, _) in &species_query {
                        ui.selectable_value(
                            &mut ui_state.selected,
                            Some(id),
                            format!("[{:?}] {}", id, name),
                        );
                    }
                });

            // delete species button
            if ui
                .add_enabled(ui_state.selected.is_some(), egui::Button::new("Delete"))
                .clicked()
            {
                commands
                    .entity(
                        ui_state
                            .selected
                            .expect("disabled button should not register presses"),
                    )
                    .despawn();
            }
        });

        if let Some(id) = ui_state.selected {
            ui.separator();
            ui.heading("Species");

            let NumAgents(mut num_agents) = species_query
                .get_component::<NumAgents>(id)
                .unwrap()
                .clone();

            let Qualities {
                mut color,
                mut speed,
                mut turn_speed,
                mut view_distance,
                mut field_of_view,
            } = species_query
                .get_component::<Qualities>(id)
                .unwrap()
                .clone();

            let mut num_agents_changed = ui.button("Randomize Agents").clicked();
            num_agents_changed |= ui
                .horizontal(|ui| {
                    let ret = ui
                        .add(egui::Slider::new(
                            &mut num_agents,
                            RangeInclusive::new(1, 1000000),
                        ))
                        .changed();
                    ui.label("Number of Agents");
                    ret
                })
                .inner;

            if num_agents_changed {
                commands.entity(id).insert(NumAgents(num_agents));
            }

            let mut qualities_changed = false;
            qualities_changed |= ui
                .horizontal(|ui| {
                    let mut color_flat = color.as_rgba_f32()[0..3].try_into().unwrap();
                    let changed = ui.color_edit_button_rgb(&mut color_flat).changed();
                    color = Color::rgb(color_flat[0], color_flat[1], color_flat[2]);
                    ui.label("Trail Color");
                    changed
                })
                .inner;

            qualities_changed |= ui
                .horizontal(|ui| {
                    let changed = ui
                        .add(egui::DragValue::new(&mut speed).speed(SPEED_DELTA))
                        .changed();
                    ui.label("Speed (units/frame)");
                    changed
                })
                .inner;

            qualities_changed |= ui
                .horizontal(|ui| {
                    let changed = ui
                        .add(egui::DragValue::new(&mut turn_speed).speed(TURN_SPEED_DELTA))
                        .changed();
                    ui.label("Turn Speed (rad/frame)");
                    changed
                })
                .inner;

            qualities_changed |= ui
                .horizontal(|ui| {
                    let changed = ui
                        .add(egui::DragValue::new(&mut view_distance).speed(VIEW_DISTANCE_DELTA))
                        .changed();
                    ui.label("View Distance (units)");
                    changed
                })
                .inner;

            qualities_changed |= ui
                .horizontal(|ui| {
                    let changed = ui
                        .add(egui::DragValue::new(&mut field_of_view).speed(FIELD_OF_VIEW_DELTA))
                        .changed();
                    ui.label("Field of View (rad)");
                    changed
                })
                .inner;

            if qualities_changed {
                commands.entity(id).insert(Qualities {
                    color,
                    speed: speed.max(0.0),
                    turn_speed: turn_speed.max(0.0),
                    view_distance: view_distance.max(0.0).min(1.0),
                    field_of_view: field_of_view.max(0.0).min(2.0 * PI),
                });
            }
        }
    });
}

fn configure_window(ui_state: Res<UiState>, mut windows: Query<&mut Window>) {
    let mut window = windows.single_mut();
    window.present_mode = if ui_state.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
}

fn main() {
    App::new()
        .init_resource::<UiState>()
        .insert_resource(WinitSettings::game())
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        // .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(slime::Plugin)
        .add_startup_system(setup)
        .add_system(configure_window)
        .add_system(draw_ui.after(EguiSet::BeginFrame))
        .run()
}
