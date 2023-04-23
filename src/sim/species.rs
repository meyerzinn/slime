use std::f32::consts::FRAC_PI_6;

use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
            BindingType, Buffer, BufferBindingType, BufferDescriptor, BufferUsages, ShaderStages,
        },
        renderer::{RenderDevice, RenderQueue},
        Extract, RenderApp, RenderSet,
    },
    utils::HashMap,
};
use bytemuck::{Pod, Zeroable};
use derive_more::From;

#[derive(Bundle)]
pub struct SpeciesBundle {
    pub num_agents: NumAgents,
    pub qualities: Qualities,
}

#[derive(Deref, Clone, Component, From)]
pub struct NumAgents(pub u32);

#[derive(Component, Clone)]
pub struct Qualities {
    pub color: Color,
    pub speed: f32,
    pub turn_speed: f32,
    pub view_distance: f32,
    pub field_of_view: f32,
}

impl Default for Qualities {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            speed: 6e-6,
            turn_speed: 1e-3,
            view_distance: 2e-2,
            field_of_view: FRAC_PI_6,
        }
    }
}

#[derive(Copy, Clone, Pod, Zeroable, Default, Component)]
#[repr(C)]
pub struct GpuQualities {
    color: Vec3,
    speed: f32,
    turn_speed: f32,
    view_distance: f32,
    field_of_view: f32,
    _padding: [f32; 5],
}

impl From<Qualities> for GpuQualities {
    fn from(qualities: Qualities) -> Self {
        Self {
            color: Vec4::from_array(qualities.color.as_rgba_f32()).truncate(),
            speed: qualities.speed,
            turn_speed: qualities.turn_speed,
            view_distance: qualities.view_distance,
            field_of_view: qualities.field_of_view,
            _padding: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Pod, Zeroable, Default)]
#[repr(C)]
struct GpuAgent {
    pos: Vec2,
    angle: f32,
    _padding: [u8; 4],
}

#[derive(Component, Deref, Clone)]
pub struct AgentsBuffer(Buffer);

#[derive(Component, Deref, Clone)]
pub struct QualitiesBuffer(Buffer);

#[derive(Resource, Deref, DerefMut, Default)]
/// Maps species to the existing agents for the species. Lives in the Render world, but the entitiy IDs are the same!
pub(crate) struct AgentsMap(HashMap<Entity, AgentsBuffer>);

#[derive(Resource, Deref, DerefMut, Default)]
struct QualitiesMap(HashMap<Entity, QualitiesBuffer>);

#[derive(Component, Debug)]
/// Marker component that indicates the agents for a species need to be intitialized.
pub struct Uninitialized;

fn render_clear_deleted(
    agents_map: Option<ResMut<AgentsMap>>,
    mut removals: RemovedComponents<NumAgents>,
) {
    if let Some(mut agents_map) = agents_map {
        for entity in removals.iter() {
            println!("removing buffers for species: {:?}", entity);
            agents_map.remove(&entity);
        }
    }
}

fn render_extract_qualities_buffer(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut qualities_map: ResMut<QualitiesMap>,
    query: Extract<Query<(Entity, Ref<Qualities>)>>,
) {
    let mut qualities_components = vec![];
    for (id, qualities) in &query {
        let qualities_buffer = qualities_map.entry(id).or_insert_with(|| {
            println!("creating new qualities buffer: {:?}", id);
            QualitiesBuffer(device.create_buffer(&BufferDescriptor {
                label: Some(&format!("[species {:?}] qualities", id)),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                size: std::mem::size_of::<GpuQualities>() as u64,
                mapped_at_creation: false,
            }))
        });
        if qualities.is_changed() {
            let gpu_qualities = GpuQualities::from(qualities.clone());
            queue.write_buffer(qualities_buffer, 0, bytemuck::bytes_of(&gpu_qualities));
        }
        qualities_components.push((id, qualities_buffer.clone()));
    }
    commands.insert_or_spawn_batch(qualities_components);
}

// extract [AgentsBuffer] for each species
fn render_extract_agents_buffer(
    mut commands: Commands,
    device: Res<RenderDevice>,
    agents_map: Option<ResMut<AgentsMap>>,
    query: Extract<Query<(Entity, Ref<NumAgents>)>>,
) {
    if let Some(mut agents_map) = agents_map {
        let mut agents_buffer_components = vec![];
        let mut uninitialized = vec![];
        for (id, num_agents) in &query {
            if num_agents.is_changed() {
                agents_map.remove(&id);
            }
            let entry = agents_map.entry(id);
            let agents_buffer = entry.or_insert_with(|| {
                uninitialized.push(id);
                println!("creating new agents buffer: {:?}", id);
                AgentsBuffer(device.create_buffer(&BufferDescriptor {
                    label: Some(&format!("[species {:?}] agents", id)),
                    size: *num_agents.clone() as u64 * (std::mem::size_of::<GpuAgent>() as u64),
                    usage: BufferUsages::STORAGE,
                    mapped_at_creation: false,
                }))
            });
            agents_buffer_components.push((id, agents_buffer.clone()));
        }
        commands.insert_or_spawn_batch(agents_buffer_components);
        commands.insert_or_spawn_batch(uninitialized.into_iter().map(|id| (id, Uninitialized)));
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct BindGroupLayout(bevy::render::render_resource::BindGroupLayout);

impl FromWorld for BindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "species::BindGroupLayout".into(),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None, // compute this dynamically
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            (std::mem::size_of::<GpuQualities>() as u64)
                                .try_into()
                                .unwrap(),
                        ),
                    },
                    count: None,
                },
            ],
        });
        Self(layout)
    }
}

#[derive(Component, Deref, DerefMut)]
pub struct BindGroup(bevy::render::render_resource::BindGroup);

fn render_queue_bind_groups(
    mut commands: Commands,
    query: Query<(Entity, &QualitiesBuffer, &AgentsBuffer)>,
    device: Res<RenderDevice>,
    layout: Res<BindGroupLayout>,
) {
    let mut components = vec![];
    for (id, qualities, agents) in &query {
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("species::BindGroup({:?})", id)),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: agents.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: qualities.as_entire_binding(),
                },
            ],
        });
        components.push((id, BindGroup(bind_group)));
    }
    commands.insert_or_spawn_batch(components);
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        // app.add_plugin(ExtractComponentPlugin::<Species>::default());
        app.sub_app_mut(RenderApp)
            .init_resource::<QualitiesMap>()
            .init_resource::<BindGroupLayout>()
            .add_system(render_queue_bind_groups.in_set(RenderSet::Queue))
            .add_system(render_extract_agents_buffer.in_schedule(ExtractSchedule))
            .add_system(render_extract_qualities_buffer.in_schedule(ExtractSchedule))
            .add_system(render_clear_deleted.in_schedule(ExtractSchedule));
    }
}
