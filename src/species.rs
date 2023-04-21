use bevy::{
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, BufferDescriptor, BufferUsages, ShaderStages,
        },
        renderer::RenderDevice,
        RenderApp, RenderSet,
    },
    utils::HashMap,
};
use bytemuck::{Pod, Zeroable};

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct GpuAgent {
    pos: Vec2,
    angle: f32,
    id: u32,
}

#[derive(Component, Deref, DerefMut)]
pub struct AgentsBindGroup(BindGroup);

#[derive(Component, Deref, DerefMut, Clone)]
pub struct Agents(Buffer);

#[derive(Resource, Deref, DerefMut, Default)]
/// Maps species to the existing agents for the species. Lives in the Render world.
struct SpeciesMap(HashMap<SpeciesId, Agents>);

#[derive(Component, Debug)]
/// Marker component that indicates the agents for a species need to be intitialized.
pub struct Uninitialized;

#[derive(Component, Copy, Clone, Deref, DerefMut, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SpeciesId(pub u64);

#[derive(Clone, Component)]
pub struct SpeciesOptions {
    pub name: String,
    pub num_agents: u32,
}

impl ExtractComponent for SpeciesOptions {
    type Query = (Entity, &'static Self);

    type Filter = ();

    type Out = (SpeciesId, Self);

    fn extract_component(item: bevy::ecs::query::QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        let (species_id, species) = item;
        // We're going to use entity ID as species ID, since it won't change in the main world.
        // We can use species ID to cache buffers in the render world.
        Some((SpeciesId(species_id.to_bits()), species.clone()))
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct AgentsBindGroupLayout(BindGroupLayout);

impl FromWorld for AgentsBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "AgentsBindGroupLayout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        Self(layout)
    }
}

// synchronize the species list from the main world to the render world
fn render_prepare_agents(
    mut commands: Commands,
    query: Query<(Entity, &SpeciesId, &SpeciesOptions)>,
    mut species: ResMut<SpeciesMap>,
    device: Res<RenderDevice>,
    simulator: Option<Res<crate::sim::Pipelines>>,
) {
    if !simulator.is_some_and(|p| p.loaded()) {
        // don't bother adding any buffers if the pipeline isn't running
        return;
    }
    {
        // add components for all species to the render world, creating buffers as needed
        let mut next_species = HashMap::new();
        for (id, &species_id, options) in &query {
            let mut entity = commands.entity(id);
            let agents = if let Some(agents) = species.get(&species_id) {
                agents.clone()
            } else {
                let buffer = device.create_buffer(&BufferDescriptor {
                    label: Some(&format!("[species {}] agents", options.name)),
                    size: options.num_agents as u64 * (std::mem::size_of::<GpuAgent>() as u64),
                    usage: BufferUsages::STORAGE,
                    mapped_at_creation: false,
                });
                entity.insert(Uninitialized);
                Agents(buffer)
            };
            next_species.insert(species_id, agents.clone());

            entity.insert(agents);
        }
        // only hold on to buffers for live species
        *species = SpeciesMap(next_species);
    }
}

fn render_queue_agent_bind_groups(
    mut commands: Commands,
    query: Query<(Entity, &SpeciesId, &Agents)>,
    device: Res<RenderDevice>,
    layout: Res<AgentsBindGroupLayout>,
) {
    for (id, species_id, agents) in &query {
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("AgentBindGroup [{:?}]", species_id)),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            }],
        });
        commands.entity(id).insert(AgentsBindGroup(bind_group));
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<SpeciesOptions>::default());

        app.sub_app_mut(RenderApp)
            .init_resource::<SpeciesMap>()
            .add_system(render_queue_agent_bind_groups.in_set(RenderSet::Queue))
            .add_system(render_prepare_agents.in_set(RenderSet::Prepare));
    }
}
