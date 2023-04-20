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
};
use bytemuck::{Pod, Zeroable};

use crate::SlimeTrailBindGroups;

const SIMULATION: &'static str = "simulation";

// @todo make this configurable
const NUM_AGENTS: u32 = 10;

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct Agent {
    position: Vec2,
    angle: f32,
    species: u32,
}

#[derive(Resource)]
pub struct GpuAgents {
    pub count: u32,
    pub buffer: Buffer,
}

impl FromWorld for GpuAgents {
    fn from_world(world: &mut World) -> Self {
        // initialize the buffer for our agents
        // todo: pull the number here somehow

        let device = world.resource::<RenderDevice>();

        let buffer = device.create_buffer(&BufferDescriptor {
            label: Some("[simulation] agents"),
            size: (NUM_AGENTS as u64) * (std::mem::size_of::<Agent>() as u64),
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        GpuAgents {
            count: NUM_AGENTS,
            buffer,
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
struct SimulationBindGroup(BindGroup);

fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<SimulationPipeline>,
    agents: Res<GpuAgents>,
    device: Res<RenderDevice>,
) {
    // todo: this does _not_ have to run every cycle??
    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("[simulation] bind_group"),
        layout: &pipeline.bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: agents.buffer.as_entire_binding(),
        }],
    });
    commands.insert_resource(SimulationBindGroup(bind_group));
}

// @todo make size, workgroup size dynamic
const WORKGROUP_SIZE: u32 = 8;

#[derive(Resource)]
struct SimulationPipeline {
    bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for SimulationPipeline {
    fn from_world(world: &mut World) -> Self {
        // initialize and copy relevant data from the game world to the render world
        let device = world.resource::<RenderDevice>();

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("[simulation] bind_group_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0, // agents
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/simulate.wgsl");
        let pipeline_cache = world.resource::<PipelineCache>();

        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("[simulation] init_pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: "init".into(),
        });

        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("[simulation] update_pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: "init".into(),
        });

        Self {
            bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        // app.add_startup_system(setup);
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<GpuAgents>()
            .init_resource::<SimulationPipeline>()
            .add_system(queue_bind_group.in_set(RenderSet::Queue));

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        // Add the compute step to the render pipeline.
        // add a node to the render graph for the simulation
        render_graph.add_node(SIMULATION, SimulationState::default());
        // make sure the simulator runs before the camera
        render_graph.add_node_edge(SIMULATION, bevy::render::main_graph::node::CAMERA_DRIVER);
    }
}

#[derive(Default, Debug)]
enum SimulationState {
    #[default]
    Loading,
    Init,
    Update(u64), // update includes # of ticks
}

impl render_graph::Node for SimulationState {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<SimulationPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        
        match self {
            SimulationState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    // initialization pipeline is cached, now we can load the update pipeline
                    *self = SimulationState::Init;
                }
            }
            SimulationState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    // update pipeline is cached, we can now advance to the update stage
                    *self = SimulationState::Update(0);
                }
            }
            // advance to the next tick
            SimulationState::Update(ticks) => *self = SimulationState::Update(*ticks + 1),
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<SimulationPipeline>();
        let agents = world.resource::<GpuAgents>();
        let slime_bind_group = world.resource::<SlimeTrailBindGroups>();
        let simulation_bind_group = world.resource::<SimulationBindGroup>();

        let mut pass = render_context
            .command_encoder()
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, &slime_bind_group[0], &[]);
        pass.set_bind_group(1, &simulation_bind_group, &[]);

        // select the pipeline based on the current state
        match *self {
            Self::Loading => { /* nothing to do while we wait for the pipelines! */ }
            Self::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(
                    (agents.count + (WORKGROUP_SIZE - 1)) / WORKGROUP_SIZE,
                    1,
                    1,
                );
            }
            Self::Update(_ticks) => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(
                    (agents.count + (WORKGROUP_SIZE - 1)) / WORKGROUP_SIZE,
                    1,
                    1,
                );
            }
        }

        Ok(())
    }
}
