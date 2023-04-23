mod blur;
mod options;
mod seed;
pub mod species;
pub mod trail;

use std::thread::sleep;

pub use options::*;
pub use species::SpeciesBundle;

use bevy::{
    prelude::*,
    render::{
        main_graph::node::CAMERA_DRIVER,
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupLayout, BindGroupLayoutDescriptor,
            CachedComputePipelineId, CachedRenderPipelineId, ColorTargetState, ColorWrites,
            ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Face, FragmentState,
            FrontFace, LoadOp, MultisampleState, Operations, PipelineCache, PolygonMode,
            PrimitiveState, PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipeline, RenderPipelineDescriptor,
        },
        renderer::RenderDevice,
        RenderApp, RenderSet,
    },
};

use crate::Framebuffers;

const SIMULATION: &str = "simulation";
const WORKGROUPS: UVec3 = UVec3::new(256, 1, 1);

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
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,

    // images (for getting texture format)
    gpu_images: Res<RenderAssets<Image>>,
    framebuffers: Res<Framebuffers>,

    // layouts
    empty_bgl: Res<EmptyBindGroupLayout>,
    species_bgl: Res<species::BindGroupLayout>,
    tex_bgl: Res<trail::TextureBindGroupLayout>,
    storage_tex_bgl: Res<trail::StorageTextureBindGroupLayout>,
    direction_bgl: Res<blur::DirectionBindGroupLayout>,
    options_bgl: Res<options::BindGroupLayout>,
    seed_bgl: Res<seed::BindGroupLayout>,
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
                    options_bgl.clone(),
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
                    tex_bgl.clone(),
                    empty_bgl.clone(),
                    seed_bgl.clone(),
                    options_bgl.clone(),
                ],
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "update".into(),
            });

            let project = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("[SimulationPipelines] project".into()),
                layout: vec![
                    species_bgl.clone(),
                    empty_bgl.clone(),
                    storage_tex_bgl.clone(),
                    empty_bgl.clone(),
                    options_bgl.clone(),
                ],
                push_constant_ranges: Vec::new(),
                shader: shader.clone(),
                shader_defs: vec![],
                entry_point: "project".into(),
            });

            let blur = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("[SimulationPipelines] blur".into()),
                layout: vec![
                    empty_bgl.clone(),
                    tex_bgl.clone(),
                    empty_bgl.clone(),
                    direction_bgl.clone(),
                    options_bgl.clone(),
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
                    // insert the map!
                    commands.init_resource::<super::species::AgentsMap>();
                    // and mark the pipeline as cached so we start rendering
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
            // let's draw this thing!

            let trail::TextureBindGroups {
                primary: tex_primary_bg,
                secondary: tex_secondary_bg,
            } = world.resource();

            let storage_tex_bg: &trail::StorageTextureBindGroup = world.resource();
            let seed_bg: &seed::BindGroup = world.resource();
            let empty_bg: &EmptyBindGroup = world.resource();
            let options_bg: &options::BindGroup = world.resource();

            let species: Vec<_> = world
                .iter_entities()
                .filter_map(|e| {
                    e.get::<species::BindGroup>()
                        .map(|species_bg| (e, species_bg))
                })
                .collect();

            {
                for (e, species_bg) in species {
                    let id = e.id();

                    let encoder = render_context.command_encoder();
                    {
                        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                            label: Some(&format!("update (species): {:?}", id)),
                        });

                        pass.set_bind_group(0, species_bg, &[]);
                        pass.set_bind_group(1, tex_primary_bg, &[]);
                        pass.set_bind_group(2, empty_bg, &[]);
                        pass.set_bind_group(3, seed_bg, &[]);
                        pass.set_bind_group(4, options_bg, &[]);

                        if e.contains::<species::Uninitialized>() {
                            // initialize agents
                            pass.set_bind_group(1, empty_bg, &[]);
                            pass.set_pipeline(init);
                        } else {
                            // update agents
                            pass.set_bind_group(1, tex_primary_bg, &[]);
                            pass.set_pipeline(update);
                        }
                        pass.dispatch_workgroups(WORKGROUPS.x, WORKGROUPS.y, WORKGROUPS.z);
                    }
                    {
                        // @todo maybe need to change shader to blend?
                        let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                            label: Some(&format!("project (species): {:?}", id)),
                        });
                        // project the agents onto the primary buffer
                        pass.set_bind_group(0, species_bg, &[]);
                        pass.set_bind_group(1, empty_bg, &[]);
                        pass.set_bind_group(2, storage_tex_bg, &[]);
                        pass.set_bind_group(3, empty_bg, &[]);
                        pass.set_bind_group(4, options_bg, &[]);
                        pass.set_pipeline(project);
                        pass.dispatch_workgroups(WORKGROUPS.x, WORKGROUPS.y, WORKGROUPS.z);
                    }
                }
            }

            let gpu_images: &RenderAssets<Image> = world.resource();
            let Framebuffers([fb_primary, fb_secondary]): &Framebuffers = world.resource();

            let blur::DirectionBindGroups {
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
                pass.set_bind_group(1, tex_primary_bg, &[]);
                pass.set_bind_group(2, empty_bg, &[]);
                pass.set_bind_group(3, horizontal, &[]);
                pass.set_bind_group(4, options_bg, &[]);
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
                pass.set_bind_group(1, tex_secondary_bg, &[]);
                pass.set_bind_group(2, empty_bg, &[]);
                pass.set_bind_group(3, vertical, &[]);
                pass.set_bind_group(4, options_bg, &[]);
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
        app.add_plugin(blur::Plugin)
            .add_plugin(species::Plugin)
            .add_plugin(seed::Plugin)
            .add_plugin(trail::Plugin)
            .add_plugin(options::Plugin);
        // add render stuff
        {
            let render_app = app.sub_app_mut(RenderApp);
            render_app
                .init_resource::<EmptyBindGroupLayout>()
                .init_resource::<EmptyBindGroup>()
                .add_system(render_queue_pipelines.in_set(RenderSet::Queue));

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
            // Add the compute step to the render pipeline.
            // add a node to the render graph for the simulation
            render_graph.add_node(SIMULATION, Simulation);
            // make sure the simulator runs before project
            render_graph.add_node_edge(SIMULATION, CAMERA_DRIVER);
        }
    }
}
