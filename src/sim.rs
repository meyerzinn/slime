use crate::{
    species::{
        Agents, Species, SpeciesBindGroup, SpeciesBindGroupLayout, SpeciesId, SpeciesOptions,
        Uninitialized,
    },
    Framebuffers,
};
use bevy::{
    prelude::*,
    render::{
        main_graph::node::CAMERA_DRIVER,
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, Buffer,
            BufferBindingType, BufferInitDescriptor, BufferUsages, CachedComputePipelineId,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, ComputePassDescriptor,
            ComputePipeline, ComputePipelineDescriptor, Face, FragmentState, FrontFace, LoadOp,
            MultisampleState, Operations, PipelineCache, PolygonMode, PrimitiveState,
            PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline,
            RenderPipelineDescriptor, ShaderStages, StorageTextureAccess, TextureFormat,
            TextureSampleType, TextureViewDimension,
        },
        renderer::{RenderDevice, RenderQueue},
        RenderApp, RenderSet,
    },
};

const SIMULATION: &str = "simulation";
const WORKGROUPS: UVec3 = UVec3::new(256, 1, 1);

#[derive(Resource, Deref, DerefMut)]
struct TextureBindGroupLayout(BindGroupLayout);

impl FromWorld for TextureBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource::<RenderDevice>();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "TextureBindGroupLayout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        Self(layout)
    }
}

#[derive(Resource, Deref, DerefMut)]
struct StorageTextureBindGroupLayout(BindGroupLayout);

impl FromWorld for StorageTextureBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource::<RenderDevice>();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "StorageTextureBindGroupLayout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::StorageTexture {
                    access: StorageTextureAccess::WriteOnly,
                    format: TextureFormat::Rgba8Unorm,
                    view_dimension: TextureViewDimension::D2,
                },
                count: None,
            }],
        });
        Self(layout)
    }
}

#[derive(Resource)]
struct StorageTextureBindGroups {
    primary: BindGroup,
    secondary: BindGroup,
}

fn render_queue_storage_texture_bind_groups(
    mut commands: Commands,
    framebuffers: Res<Framebuffers>,
    gpu_images: Res<RenderAssets<Image>>,
    layout: Res<StorageTextureBindGroupLayout>,
    device: Res<RenderDevice>,
) {
    let [primary, secondary] = [0, 1].map(|i| {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("StorageTextureBindGroup_{}", i)),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&gpu_images[&framebuffers[i]].texture_view),
            }],
        })
    });
    commands.insert_resource(StorageTextureBindGroups { primary, secondary });
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
                visibility: ShaderStages::FRAGMENT,
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
struct TextureBindGroups {
    primary: BindGroup,
    secondary: BindGroup,
}

fn render_queue_textures_bind_group(
    mut commands: Commands,
    framebuffers: Res<Framebuffers>,
    gpu_images: Res<RenderAssets<Image>>,
    layout: Res<TextureBindGroupLayout>,
    device: Res<RenderDevice>,
) {
    let [primary, secondary] = [0, 1].map(|i| {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("simulate::TextureBindGroup_{}", i)),
            layout: &layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&gpu_images[&framebuffers[i]].texture_view),
            }],
        })
    });
    commands.insert_resource(TextureBindGroups { primary, secondary });
}

#[derive(Resource, Deref, DerefMut)]
struct EmptyBindGroupLayout(BindGroupLayout);

impl FromWorld for EmptyBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource::<RenderDevice>();
        let dummy_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "dummy".into(),
            entries: &[],
        });
        Self(dummy_layout)
    }
}

#[derive(Resource, Deref, DerefMut)]
struct EmptyBindGroup(BindGroup);

impl FromWorld for EmptyBindGroup {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource();
        let layout: &EmptyBindGroupLayout = world.resource();
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "dummy".into(),
            layout,
            entries: &[],
        });
        Self(bind_group)
    }
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

#[derive(Resource, Deref, DerefMut)]
struct SeedBuffer(Buffer);

impl FromWorld for SeedBuffer {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource();
        let seed: u32 = rand::random();
        let buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: "random seed (u32)".into(),
            contents: bytemuck::bytes_of(&seed),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        Self(buffer)
    }
}

fn render_prepare_update_random_seed(queue: Res<RenderQueue>, buffer: Res<SeedBuffer>) {
    let seed: u32 = rand::random();
    queue.write_buffer(&buffer, 0, bytemuck::bytes_of(&seed));
}

#[derive(Resource, Deref, DerefMut)]
struct SeedBindGroupLayout(BindGroupLayout);

impl FromWorld for SeedBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource::<RenderDevice>();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "SeedBindGroupLayout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some((std::mem::size_of::<u32>() as u64).try_into().unwrap()),
                },
                count: None,
            }],
        });
        Self(layout)
    }
}

#[derive(Resource, Deref, DerefMut)]
struct SeedBindGroup(BindGroup);

impl FromWorld for SeedBindGroup {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource();
        let layout: &SeedBindGroupLayout = world.resource();
        let buffer: &SeedBuffer = world.resource();
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "SeedBindGroup".into(),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Self(bind_group)
    }
}

#[derive(Resource)]
pub enum Pipelines {
    Pending {
        init: CachedComputePipelineId,
        update: CachedComputePipelineId,
        project: CachedComputePipelineId,
        blur: CachedRenderPipelineId,
    },
    Cached {
        init: ComputePipeline,
        update: ComputePipeline,
        project: ComputePipeline,
        blur: RenderPipeline,
    },
}

impl Pipelines {
    /// Returns whether the pipelines are ready to run during this render pass.
    pub fn loaded(&self) -> bool {
        matches!(self, Self::Cached { .. })
    }
}

#[allow(clippy::too_many_arguments)]
fn render_queue_pipelines(
    mut commands: Commands,
    pipelines: Option<Res<Pipelines>>,
    empty_bgl: Res<EmptyBindGroupLayout>,
    species_bgl: Res<SpeciesBindGroupLayout>,
    texture_bgl: Res<TextureBindGroupLayout>,
    stbgl: Res<StorageTextureBindGroupLayout>,
    dbgl: Res<DirectionBindGroupLayout>,
    seed_bgl: Res<SeedBindGroupLayout>,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,

    gpu_images: Res<RenderAssets<Image>>,
    framebuffers: Res<Framebuffers>,
) {
    match pipelines {
        None => {
            let shader = asset_server.load("shaders/simulate.wgsl");

            let init = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("[SimulationPipelines] init_pipeline".into()),
                layout: vec![
                    species_bgl.clone(),
                    empty_bgl.clone(),
                    empty_bgl.clone(),
                    seed_bgl.clone(),
                ],
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "init".into(),
            });

            let update = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("[SimulationPipelines] update".into()),
                layout: vec![
                    species_bgl.clone(),
                    texture_bgl.clone(),
                    empty_bgl.clone(),
                    seed_bgl.clone(),
                ],
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "update".into(),
            });

            let project = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("[SimulationPipelines] project".into()),
                layout: vec![species_bgl.clone(), empty_bgl.clone(), stbgl.clone()],
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "project".into(),
            });

            let blur = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("[SimulationPipelines] blur".into()),
                layout: vec![
                    empty_bgl.clone(),
                    texture_bgl.clone(),
                    empty_bgl.clone(),
                    dbgl.clone(),
                ],
                push_constant_ranges: Vec::new(),
                vertex:
                    bevy::core_pipeline::fullscreen_vertex_shader::fullscreen_shader_vertex_state(),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleStrip,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: Some(Face::Back),
                    unclipped_depth: false,
                    polygon_mode: PolygonMode::Fill,
                    // @todo check if we can do conservative rasterization
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(FragmentState {
                    shader,
                    shader_defs: Vec::new(),
                    entry_point: "blur_fragment".into(),
                    targets: vec![Some(ColorTargetState {
                        format: gpu_images[&framebuffers[0]].texture_format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
            });

            commands.insert_resource(Pipelines::Pending {
                init,
                update,
                project,
                blur,
            })
        }
        Some(res) => match *res {
            Pipelines::Pending {
                init,
                update,
                project,
                blur,
            } => {
                if let (Some(init), Some(update), Some(project), Some(blur)) = (
                    pipeline_cache.get_compute_pipeline(init),
                    pipeline_cache.get_compute_pipeline(update),
                    pipeline_cache.get_compute_pipeline(project),
                    pipeline_cache.get_render_pipeline(blur),
                ) {
                    commands.insert_resource(Pipelines::Cached {
                        init: init.clone(),
                        update: update.clone(),
                        project: project.clone(),
                        blur: blur.clone(),
                    });
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

            let TextureBindGroups {
                primary: tbg_p,
                secondary: tbg_s,
            } = world.resource();

            let StorageTextureBindGroups {
                primary: stbg_p,
                secondary: _stbg_s,
            } = world.resource();

            let seed_bg: &SeedBindGroup = world.resource();

            let EmptyBindGroup(empty_bg) = world.resource();

            let species: Vec<_> = world
                .iter_entities()
                .filter(|e| {
                    e.contains::<Agents>()
                        && e.contains::<Species>()
                        && e.contains::<SpeciesId>()
                        && e.contains::<SpeciesOptions>()
                        && e.contains::<SpeciesBindGroup>()
                })
                .collect();

            {
                for sp in species {
                    let encoder = render_context.command_encoder();
                    let id: &SpeciesId = sp.get().unwrap();
                    let so: &SpeciesOptions = sp.get().unwrap();
                    let species_bg: &SpeciesBindGroup = sp.get().unwrap();
                    {
                        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                            label: Some(&format!("simulation (species): {:?} ({})", id, so.name)),
                        });

                        pass.set_bind_group(0, species_bg, &[]);
                        pass.set_bind_group(2, &empty_bg, &[]);
                        pass.set_bind_group(3, seed_bg, &[]);
                        // pass.set_bind_group(4, , offsets)
                        if sp.contains::<Uninitialized>() {
                            // initialize agents
                            pass.set_bind_group(1, &empty_bg, &[]);
                            pass.set_pipeline(init);
                        } else {
                            // update agents
                            pass.set_bind_group(1, tbg_p, &[]);
                            pass.set_pipeline(update);
                        }
                        pass.dispatch_workgroups(WORKGROUPS.x, WORKGROUPS.y, WORKGROUPS.z);
                    }
                    {
                        // @todo maybe need to change shader to blend?
                        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                            label: Some(&format!("project (species): {:?} ({})", id, so.name)),
                        });
                        // project the agents onto the primary buffer
                        pass.set_bind_group(0, species_bg, &[]);
                        pass.set_bind_group(1, empty_bg, &[]);
                        pass.set_bind_group(2, stbg_p, &[]);
                        pass.set_pipeline(project);
                        pass.dispatch_workgroups(WORKGROUPS.x, WORKGROUPS.y, WORKGROUPS.z);
                    }
                }
            }

            let gpu_images: &RenderAssets<Image> = world.resource();
            let Framebuffers([fb_primary, fb_secondary]): &Framebuffers = world.resource();

            let DirectionBindGroups {
                horizontal,
                vertical,
            } = &world.resource::<_>();

            // horizontal blur pass
            {
                let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                    label: "blur (horizontal)".into(),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &gpu_images[fb_secondary].texture_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::RED.into()),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });
                pass.set_bind_group(0, empty_bg, &[]);
                pass.set_bind_group(1, tbg_p, &[]);
                pass.set_bind_group(2, empty_bg, &[]);
                pass.set_bind_group(3, horizontal, &[]);
                pass.set_render_pipeline(blur);
                pass.draw(0..4, 0..1);
            }

            // vertical blur pass
            {
                let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                    label: "blur (vertical)".into(),
                    color_attachments: &[Some(RenderPassColorAttachment {
                        view: &gpu_images[fb_primary].texture_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(Color::BLUE.into()),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });
                pass.set_bind_group(0, empty_bg, &[]);
                pass.set_bind_group(1, tbg_s, &[]);
                pass.set_bind_group(2, empty_bg, &[]);
                pass.set_bind_group(3, vertical, &[]);
                pass.set_render_pipeline(blur);
                pass.draw(0..4, 0..1);
            }
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
                .init_resource::<SpeciesBindGroupLayout>()
                .init_resource::<DirectionBindGroupLayout>()
                .init_resource::<DirectionBindGroups>()
                .init_resource::<EmptyBindGroupLayout>()
                .init_resource::<EmptyBindGroup>()
                .init_resource::<SeedBuffer>()
                .init_resource::<SeedBindGroupLayout>()
                .init_resource::<SeedBindGroup>()
                .init_resource::<StorageTextureBindGroupLayout>()
                .init_resource::<TextureBindGroupLayout>()
                .add_system(render_prepare_update_random_seed.in_set(RenderSet::Prepare))
                .add_system(render_queue_pipelines.in_set(RenderSet::Queue))
                .add_system(render_queue_textures_bind_group.in_set(RenderSet::Queue))
                .add_system(render_queue_storage_texture_bind_groups.in_set(RenderSet::Queue));

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
            // Add the compute step to the render pipeline.
            // add a node to the render graph for the simulation
            render_graph.add_node(SIMULATION, Simulation);
            // make sure the simulator runs before project
            render_graph.add_node_edge(SIMULATION, CAMERA_DRIVER);
        }
    }
}
