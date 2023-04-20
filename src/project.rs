use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::{
            BlendComponent, BlendState, CachedPipelineState, CachedRenderPipelineId,
            ColorTargetState, ColorWrites, Face, FragmentState, FrontFace, LoadOp,
            MultisampleState, Operations, PipelineCache, PolygonMode, PrimitiveState,
            PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, VertexState,
        },
        RenderApp,
    },
};

pub const PROJECT: &'static str = "project";

#[derive(Resource)]
struct Pipeline {
    render_pipeline: CachedRenderPipelineId,
}

// impl FromWorld for Pipeline {
//     fn from_world(world: &mut bevy::prelude::World) -> Self {

//     }
// }

#[derive(Default)]
enum Projection {
    #[default]
    WaitingForImages,
    Loading,
    Running,
}

impl Projection {
    fn is_running(&self) -> bool {
        match self {
            Projection::Running => true,
            _ => false,
        }
    }
}

// fn setup(
//     mut commands: Commands,
//     images: Res<super::TrailImages>,
//     gpu_images: Res<RenderAssets<Image>>,
//     textures_bind_group_layout: Res<super::TexturesBindGroupLayout>,
//     asset_server: Res<AssetServer>,
//     pipeline_cache: Res<PipelineCache>,
// ) {

// }

impl render_graph::Node for Projection {
    fn update(&mut self, world: &mut bevy::prelude::World) {
        let pipeline_cache = world.resource::<PipelineCache>();
        let textures_bind_group_layout = world.resource::<super::TexturesBindGroupLayout>();

        match self {
            Self::WaitingForImages => {
                if let Some(images) = world.get_resource::<super::TrailImages>() {
                    let format = world.resource::<RenderAssets<Image>>()[&images[0]].texture_format;

                    let shader = world.resource::<AssetServer>().load("shaders/project.wgsl");

                    let render_pipeline =
                        pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                            label: Some("[ProjectPipeline] render_pipeline".into()),
                            layout: vec![
                                // @todo: need to split this into two different bindings?
                                // or make a totally separate binding for project
                                textures_bind_group_layout.0.clone(), // unused
                            ],
                            push_constant_ranges: Vec::new(),
                            vertex: VertexState {
                                shader: shader.clone(),
                                shader_defs: vec![],
                                entry_point: "vs_main".into(),
                                buffers: vec![],
                            },
                            primitive: PrimitiveState {
                                topology: PrimitiveTopology::TriangleStrip,
                                strip_index_format: None,
                                front_face: FrontFace::Ccw,
                                cull_mode: Some(Face::Back),
                                polygon_mode: PolygonMode::Fill,
                                unclipped_depth: false,
                                conservative: false,
                            },
                            depth_stencil: None,
                            multisample: MultisampleState {
                                count: 1,
                                mask: !0,
                                alpha_to_coverage_enabled: false,
                            },
                            fragment: Some(FragmentState {
                                shader: shader.clone(),
                                shader_defs: vec![],
                                entry_point: "fs_main".into(),
                                targets: vec![Some(ColorTargetState {
                                    format,
                                    blend: None,
                                    write_mask: ColorWrites::ALL,
                                })],
                            }),
                        });

                    world.insert_resource(Pipeline { render_pipeline });
                    *self = Self::Loading;
                }
            }
            Self::Loading => {
                let pipeline = world.resource::<Pipeline>();
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_render_pipeline_state(pipeline.render_pipeline)
                {
                    // pipeline is cached, we're ready to roll!
                    *self = Self::Running;
                }
            }

            Self::Running => { /*no updates needed */ }
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &bevy::prelude::World,
    ) -> Result<(), render_graph::NodeRunError> {
        if !self.is_running() {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<Pipeline>();
        let render_pipeline = pipeline_cache
            .get_render_pipeline(pipeline.render_pipeline)
            .unwrap();

        let gpu_images = world.resource::<RenderAssets<Image>>();
        let textures = world.resource::<super::TrailImages>();
        let view = &gpu_images[&textures[0]].texture_view;

        let trail_bind_groups = world.resource::<super::TrailBindGroups>();

        let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("[ProjectionState] pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        // pass.set_bind_group(0, &trail_bind_groups[0], &[]);
        pass.set_render_pipeline(render_pipeline);
        pass.draw(0..4, 0..1);

        Ok(())
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        {
            let render_app = app.sub_app_mut(RenderApp);
            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
            // Add the compute step to the render pipeline.
            // add a node to the render graph for the simulation
            render_graph.add_node(PROJECT, Projection::default());
            // make sure project runs before the camera
            render_graph.add_node_edge(PROJECT, bevy::render::main_graph::node::CAMERA_DRIVER);
        }
    }
}
