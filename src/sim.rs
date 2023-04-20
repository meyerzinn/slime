use crate::{SpeciesId, SpeciesOptions};
use bevy::{
    prelude::*,
    render::{
        render_graph::{self, RenderGraph},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer,
            BufferBindingType, BufferDescriptor, BufferUsages, CachedComputePipelineId,
            CachedPipelineState, ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache,
            ShaderStages,
        },
        renderer::RenderDevice,
        RenderApp, RenderSet,
    },
    utils::HashMap,
};
use bytemuck::{Pod, Zeroable};

const SIMULATION: &'static str = "simulation";
const WORKGROUP_SIZE: UVec3 = UVec3::new(256, 1, 1);

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct GpuAgent {
    pos: Vec2,
    angle: f32,
    id: u32,
}

#[derive(Component, Deref, DerefMut, Clone)]
struct Agents(Buffer);

#[derive(Resource, Deref, DerefMut, Default)]
/// Maps species to the existing agents for the species. Lives in the Render world.
struct SpeciesMap(HashMap<SpeciesId, Agents>);

#[derive(Component)]
/// Marker component that indicates the agents for a species need to be intitialized.
struct Uninitialized;

#[derive(Component, Deref, DerefMut)]
struct AgentsBindGroup(BindGroup);

#[derive(Resource, Deref, DerefMut)]
struct AgentsBindGroupLayout(BindGroupLayout);

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
    query: Query<(Entity, &SpeciesId, &crate::SpeciesOptions)>,
    mut species: ResMut<SpeciesMap>,
    device: Res<RenderDevice>,
) {
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

            commands.entity(id).insert(agents);
        }
        // only hold on to buffers for live species
        *species = SpeciesMap(next_species);
    }
}

fn render_queue_bind_groups(
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

#[derive(Resource)]
struct SimulationPipelines {
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for SimulationPipelines {
    fn from_world(world: &mut World) -> Self {
        let agents_bind_group_layout = world.resource::<AgentsBindGroupLayout>();
        let textures_bind_group_layout = world.resource::<super::TexturesBindGroupLayout>();

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/simulate.wgsl");

        let pipeline_cache = world.resource::<PipelineCache>();

        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("[SimulationPipelines] init_pipeline".into()),
            layout: vec![
                textures_bind_group_layout.0.clone(), // unused
                agents_bind_group_layout.0.clone(),
            ],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: "init".into(),
        });

        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("[SimulationPipelines] update_pipeline".into()),
            layout: vec![
                textures_bind_group_layout.0.clone(),
                agents_bind_group_layout.0.clone(),
            ],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: "update".into(),
        });

        Self {
            init_pipeline,
            update_pipeline,
        }
    }
}

#[derive(Default)]
enum Simulation {
    #[default]
    LoadingInitializationPipeline,
    LoadingUpdatePipeline,
    Running,
}

impl render_graph::Node for Simulation {
    fn update(&mut self, world: &mut World) {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<SimulationPipelines>();

        match self {
            Self::LoadingInitializationPipeline => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipelines.init_pipeline)
                {
                    // initialization pipeline is cached, now we can load the update pipeline
                    *self = Self::LoadingUpdatePipeline;
                }
            }
            Self::LoadingUpdatePipeline => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipelines.update_pipeline)
                {
                    // update pipeline is cached, we can now advance to the update stage
                    *self = Self::Running;
                }
            }
            // advance to the next tick
            Self::Running => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<SimulationPipelines>();
        let texture_bind_groups = world.resource::<super::TrailBindGroups>();

        let init_pipeline = pipeline_cache
            .get_compute_pipeline(pipelines.init_pipeline)
            .unwrap();

        let update_pipeline = pipeline_cache
            .get_compute_pipeline(pipelines.update_pipeline)
            .unwrap();

        let species: Vec<_> = world
            .iter_entities()
            .filter(|e| {
                e.contains::<super::SpeciesId>()
                    && e.contains::<super::SpeciesOptions>()
                    && e.contains::<AgentsBindGroup>()
            })
            .collect();

        let encoder = render_context.command_encoder();

        // initialize all agents for new species
        for sp in species {
            let id = sp.get::<SpeciesId>().unwrap();
            let so = sp.get::<SpeciesOptions>().unwrap();
            let bg = sp.get::<AgentsBindGroup>().unwrap();
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(&format!("species: {:?} ({})", id, so.name)),
            });
            pass.set_bind_group(0, &texture_bind_groups[0], &[]); // unused
            pass.set_bind_group(1, bg, &[]);
            if sp.contains::<Uninitialized>() {
                // initialize agents
                pass.set_pipeline(init_pipeline);
            } else {
                // update agents
                pass.set_pipeline(update_pipeline);
            }
            pass.dispatch_workgroups(WORKGROUP_SIZE.x, WORKGROUP_SIZE.y, WORKGROUP_SIZE.z);
        }
        Ok(())
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        {
            let render_app = app.sub_app_mut(RenderApp);
            render_app.init_resource::<SpeciesMap>();
            render_app.init_resource::<AgentsBindGroupLayout>();
            render_app.init_resource::<SimulationPipelines>();
            render_app.add_system(render_prepare_agents.in_set(RenderSet::Prepare));
            render_app.add_system(render_queue_bind_groups.in_set(RenderSet::Queue));

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
            // Add the compute step to the render pipeline.
            // add a node to the render graph for the simulation
            render_graph.add_node(SIMULATION, Simulation::default());
            // make sure the simulator runs before project
            render_graph.add_node_edge(SIMULATION, super::project::PROJECT);
        }
    }
}
