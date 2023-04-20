use crate::{
    species::{AgentsBindGroup, AgentsBindGroupLayout, SpeciesId, SpeciesOptions, Uninitialized},
    SecondaryFramebuffer,
};
use bevy::{
    prelude::*,
    render::{
        main_graph::node::CAMERA_DRIVER,
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            BufferBindingType, BufferInitDescriptor, BufferUsages, CachedComputePipelineId,
            ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, PipelineCache,
            ShaderStages, StorageTextureAccess, TextureFormat, TextureSampleType,
            TextureViewDimension,
        },
        renderer::RenderDevice,
        RenderApp, RenderSet,
    },
};
use std::ops::Deref;

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
struct DirectionBindGroupLayout(BindGroupLayout);

impl FromWorld for DirectionBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource::<RenderDevice>();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "DirectionBindGroupLayout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        (std::mem::size_of::<IVec2>() as u64).try_into().unwrap(),
                    ),
                },
                count: None,
            }],
        });
        Self(layout)
    }
}

#[derive(Resource)]
struct TexturesBindGroups {
    primary: BindGroup,
    secondary: BindGroup,
}

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
    let [primary, secondary] = [0, 1].map(|i| {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("simulate::TexturesBindGroup_{}", i)),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: i,
                    resource: BindingResource::TextureView(primary),
                },
                BindGroupEntry {
                    binding: 1 - i,
                    resource: BindingResource::TextureView(secondary),
                },
            ],
        })
    });
    commands.insert_resource(TexturesBindGroups { primary, secondary });
}

#[derive(Resource)]
struct DirectionBindGroups {
    horizontal: BindGroup,
    vertical: BindGroup,
}

impl FromWorld for DirectionBindGroups {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource::<RenderDevice>();
        let layout = world.resource::<DirectionBindGroupLayout>();

        let vertical = device.create_bind_group(&BindGroupDescriptor {
            label: "DirectionBindGroups_vertical".into(),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: device
                    .create_buffer_with_data(&BufferInitDescriptor {
                        label: "IVec2{x: 0, y: 1}".into(),
                        contents: bytemuck::bytes_of(&IVec2::new(0, 1)),
                        usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                    })
                    .as_entire_binding(),
            }],
        });

        let horizontal = device.create_bind_group(&BindGroupDescriptor {
            label: "DirectionBindGroups_horizontal".into(),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: device
                    .create_buffer_with_data(&BufferInitDescriptor {
                        label: "IVec2{x: 1, y: 0}".into(),
                        contents: bytemuck::bytes_of(&IVec2::new(1, 0)),
                        usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
                    })
                    .as_entire_binding(),
            }],
        });

        Self {
            horizontal,
            vertical,
        }
    }
}

#[derive(Resource)]
enum Pipelines {
    Pending {
        init: CachedComputePipelineId,
        update: CachedComputePipelineId,
        project: CachedComputePipelineId,
        blur: CachedComputePipelineId,
    },
    Cached {
        init: ComputePipeline,
        update: ComputePipeline,
        project: ComputePipeline,
        blur: ComputePipeline,
    },
}

fn render_queue_prepare_pipelines(
    mut commands: Commands,
    pipelines: Option<Res<Pipelines>>,
    agents_bind_group_layout: Res<AgentsBindGroupLayout>,
    textures_bind_group_layout: Res<TexturesBindGroupLayout>,
    direction_bind_group_layout: Res<DirectionBindGroupLayout>,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    match pipelines {
        None => {
            let shader = asset_server.load("shaders/simulate.wgsl");

            let init = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("[SimulationPipelines] init_pipeline".into()),
                layout: vec![agents_bind_group_layout.deref().deref().clone()],
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "init".into(),
            });

            let mut layout = vec![
                agents_bind_group_layout.deref().deref().clone(),
                textures_bind_group_layout.deref().deref().clone(),
            ];

            let update = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("[SimulationPipelines] update".into()),
                layout: layout.clone(),
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "update".into(),
            });

            let project = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("[SimulationPipelines] project".into()),
                layout: layout.clone(),
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "project".into(),
            });

            layout.push(direction_bind_group_layout.deref().deref().clone());

            let blur = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("[SimulationPipelines] blur".into()),
                layout: layout.clone(),
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "blur".into(),
            });

            commands.insert_resource(Pipelines::Pending {
                init,
                update,
                project,
                blur,
            })
        }
        Some(res) => match *res.deref() {
            Pipelines::Pending {
                init,
                update,
                project,
                blur,
            } => {
                match (
                    pipeline_cache.get_compute_pipeline(init),
                    pipeline_cache.get_compute_pipeline(update),
                    pipeline_cache.get_compute_pipeline(project),
                    pipeline_cache.get_compute_pipeline(blur),
                ) {
                    (Some(init), Some(update), Some(project), Some(blur)) => commands
                        .insert_resource(Pipelines::Cached {
                            init: init.clone(),
                            update: update.clone(),
                            project: project.clone(),
                            blur: blur.clone(),
                        }),
                    _ => { /* pipelines are still pending */ }
                }
            }
            Pipelines::Cached { .. } => { /* pipelines already valid */ }
        },
    }
}

struct Simulation;

impl render_graph::Node for Simulation {
    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        if let Some(Pipelines::Cached {
            init,
            update,
            project,
            blur,
        }) = world.get_resource::<Pipelines>()
        {
            // pipelines are created & cached
            let TexturesBindGroups { primary, secondary } = world.resource::<TexturesBindGroups>();

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
                let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some(&format!("simulation (species): {:?} ({})", id, so.name)),
                });

                pass.set_bind_group(0, bg, &[]);
                pass.set_bind_group(1, primary, &[]);

                if sp.contains::<Uninitialized>() {
                    // initialize agents
                    pass.set_pipeline(init);
                } else {
                    // update agents
                    pass.set_pipeline(update);
                }
                pass.dispatch_workgroups(WORKGROUPS.x, WORKGROUPS.y, WORKGROUPS.z);

                // project the agents onto the primary buffer
                pass.set_bind_group(1, secondary, &[]);
                pass.set_pipeline(project);
                pass.dispatch_workgroups(WORKGROUPS.x, WORKGROUPS.y, WORKGROUPS.z);

                let DirectionBindGroups {
                    horizontal,
                    vertical,
                } = &world.resource::<_>();

                pass.set_pipeline(blur);

                // apply first blur pass to primary onto secondary
                pass.set_bind_group(1, primary, &[]);
                pass.set_bind_group(2, horizontal, &[]);
                pass.dispatch_workgroups(WORKGROUPS.x, WORKGROUPS.y, WORKGROUPS.z);

                // apply second blur pass to secondary onto primary
                pass.set_bind_group(1, secondary, &[]);
                pass.set_bind_group(2, vertical, &[]);
                pass.dispatch_workgroups(WORKGROUPS.x, WORKGROUPS.y, WORKGROUPS.z);
            }

            // no need to clear?

            // let gpu_images = world.resource::<RenderAssets<Image>>();
            // let secondary = &gpu_images[world.resource::<SecondaryFramebuffer>().deref()];
            // let range_all = ImageSubresourceRange {
            //     aspect: TextureAspect::All,
            //     base_mip_level: 0,
            //     mip_level_count: secondary.mip_level_count.try_into().ok(),
            //     base_array_layer: 0,
            //     array_layer_count: None,
            // };
        }
        Ok(())
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        {
            let render_app = app.sub_app_mut(RenderApp);
            render_app
                .init_resource::<TexturesBindGroupLayout>()
                .init_resource::<AgentsBindGroupLayout>()
                .init_resource::<DirectionBindGroupLayout>()
                .init_resource::<DirectionBindGroups>()
                .add_system(render_queue_prepare_pipelines.in_set(RenderSet::Queue))
                .add_system(render_queue_textures_bind_group.in_set(RenderSet::Queue));

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
            // Add the compute step to the render pipeline.
            // add a node to the render graph for the simulation
            render_graph.add_node(SIMULATION, Simulation);
            // make sure the simulator runs before project
            render_graph.add_node_edge(SIMULATION, CAMERA_DRIVER);
        }
    }
}
