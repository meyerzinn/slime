use std::ops::Deref;

use crate::species::{
    AgentsBindGroup, AgentsBindGroupLayout, SpeciesId, SpeciesOptions, Uninitialized,
};
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            CachedComputePipelineId, CachedPipelineState, ComputePassDescriptor,
            ComputePipelineDescriptor, PipelineCache, ShaderStages, StorageTextureAccess,
            TextureFormat, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        RenderApp, RenderSet,
    },
};

const SIMULATION: &'static str = "simulation";
const WORKGROUPS: UVec3 = UVec3::new(256, 1, 1);

#[derive(Resource, Deref, DerefMut)]
struct TexturesBindGroupLayout(BindGroupLayout);

impl FromWorld for TexturesBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource::<RenderDevice>();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "TexturesBindGroupLayout".into(),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba8Unorm,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });
        Self(layout)
    }
}

#[derive(Resource, Deref, DerefMut)]
struct TexturesBindGroup(BindGroup);

fn render_queue_textures_bind_group(
    mut commands: Commands,
    primary: Res<super::PrimaryFramebuffer>,
    secondary: Res<super::SecondaryFramebuffer>,
    gpu_images: Res<RenderAssets<Image>>,
    layout: Res<TexturesBindGroupLayout>,
    device: Res<RenderDevice>,
) {
    let primary = &gpu_images[&primary].texture_view;
    let secondary = &gpu_images[&secondary].texture_view;
    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: "simulate::TexturesBindGroup".into(),
        layout: &layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(primary),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(secondary),
            },
        ],
    });
    commands.insert_resource(TexturesBindGroup(bind_group));
}

#[derive(Resource)]
struct SimulationPipelines {
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
    project_pipeline: CachedComputePipelineId,
}

impl FromWorld for SimulationPipelines {
    fn from_world(world: &mut World) -> Self {
        let agents_bind_group_layout = world.resource::<AgentsBindGroupLayout>();
        let textures_bind_group_layout = world.resource::<TexturesBindGroupLayout>();

        let shader = world
            .resource::<AssetServer>()
            .load("shaders/simulate.wgsl");

        let pipeline_cache = world.resource::<PipelineCache>();

        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("[SimulationPipelines] init_pipeline".into()),
            layout: vec![
                agents_bind_group_layout.deref().clone(),
                // textures_bind_group_layout.deref().clone(), // unused
            ],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: "init".into(),
        });

        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("[SimulationPipelines] update_pipeline".into()),
            layout: vec![
                agents_bind_group_layout.deref().clone(),
                textures_bind_group_layout.deref().clone(),
            ],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: "update".into(),
        });

        let project_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("[SimulationPipelines] project_pipeline".into()),
            layout: vec![
                agents_bind_group_layout.deref().clone(),
                textures_bind_group_layout.deref().clone(),
            ],
            push_constant_ranges: Vec::new(),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: "project".into(),
        });

        Self {
            init_pipeline,
            update_pipeline,
            project_pipeline,
        }
    }
}

#[derive(Default)]
enum Simulation {
    #[default]
    LoadingInitializationPipeline,
    LoadingUpdatePipeline,
    LoadingProjectPipeline,
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
                    *self = Self::LoadingProjectPipeline;
                }
            }
            Self::LoadingProjectPipeline => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipelines.project_pipeline)
                {
                    // project pipeline is cached, we can now advance
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
        let textures_bind_group = &world.resource::<TexturesBindGroup>();

        let init_pipeline = pipeline_cache
            .get_compute_pipeline(pipelines.init_pipeline)
            .unwrap();

        let update_pipeline = pipeline_cache
            .get_compute_pipeline(pipelines.update_pipeline)
            .unwrap();

        let project_pipeline = pipeline_cache
            .get_compute_pipeline(pipelines.project_pipeline)
            .unwrap();

        let species: Vec<_> = world
            .iter_entities()
            .filter(|e| {
                e.contains::<SpeciesId>()
                    && e.contains::<SpeciesOptions>()
                    && e.contains::<AgentsBindGroup>()
            })
            .collect();

        let encoder = render_context.command_encoder();

        for sp in species {
            let id = sp.get::<SpeciesId>().unwrap();
            let so = sp.get::<SpeciesOptions>().unwrap();
            let bg = sp.get::<AgentsBindGroup>().unwrap();
            let mut first_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(&format!("first pass: {:?} ({})", id, so.name)),
            });
            first_pass.set_bind_group(0, bg, &[]);
            first_pass.set_bind_group(1, textures_bind_group, &[]); // unused
            if sp.contains::<Uninitialized>() {
                // initialize agents
                first_pass.set_pipeline(init_pipeline);
            } else {
                // update agents
                first_pass.set_pipeline(update_pipeline);
            }
            first_pass.dispatch_workgroups(WORKGROUPS.x, WORKGROUPS.y, WORKGROUPS.z);
            first_pass.set_pipeline(&project_pipeline);
            first_pass.dispatch_workgroups(WORKGROUPS.x, WORKGROUPS.y, WORKGROUPS.z);

            // let mut second_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
            //     label: Some(&format!("project pass: {:?} ({})", id, so.name)),
            // });
            // second_pass.set_bind_group(0, bind_group, offsets)
        }
        Ok(())
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        {
            let render_app = app.sub_app_mut(RenderApp);
            render_app.init_resource::<TexturesBindGroupLayout>();
            render_app.init_resource::<AgentsBindGroupLayout>();
            render_app.init_resource::<SimulationPipelines>();
            render_app.add_system(render_queue_textures_bind_group.in_set(RenderSet::Queue));

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
            // Add the compute step to the render pipeline.
            // add a node to the render graph for the simulation
            render_graph.add_node(SIMULATION, Simulation::default());
            // make sure the simulator runs before project
            render_graph.add_node_edge(SIMULATION, super::blur::BLUR);
        }
    }
}
