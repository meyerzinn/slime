use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            CachedPipelineState, CachedRenderPipelineId, ColorTargetState, ColorWrites, Face,
            FragmentState, FrontFace, LoadOp, MultisampleState, Operations, PipelineCache,
            PolygonMode, PrimitiveState, PrimitiveTopology, RenderPassColorAttachment,
            RenderPassDescriptor, RenderPipelineDescriptor, ShaderStages, TextureSampleType,
            TextureViewDimension, VertexState,
        },
        renderer::RenderDevice,
        RenderApp, RenderSet,
    },
};

pub const BLUR: &'static str = "blur";

#[derive(Resource, Deref)]
struct TextureBindGroupLayout(BindGroupLayout);

impl FromWorld for TextureBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("blur::TextureBindGroupLayout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        Self(bind_group_layout)
    }
}

#[derive(Resource, Deref, DerefMut)]
struct TextureBindGroup(BindGroup);

fn render_queue_texture_bind_group(
    mut commands: Commands,
    layout: Res<TextureBindGroupLayout>,
    device: Res<RenderDevice>,
    target: Res<super::PrimaryFramebuffer>,
    gpu_images: Res<RenderAssets<Image>>,
) {
    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: "blur::render_queue_texture_bind_group".into(),
        layout: &layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(&gpu_images[&target].texture_view.clone()),
        }],
    });
    commands.insert_resource(TextureBindGroup(bind_group));
}

#[derive(Resource)]
struct Pipeline {
    render_pipeline: CachedRenderPipelineId,
}

#[derive(Default)]
enum BlurPass {
    #[default]
    WaitingForImages,
    Loading,
    Rendering(bool), // boolean represents whether there has been an update since the last draw
}

impl BlurPass {
    fn should_draw(&self) -> bool {
        match self {
            Self::Rendering(true) => true,
            _ => false,
        }
    }
}

impl render_graph::Node for BlurPass {
    fn update(&mut self, world: &mut bevy::prelude::World) {
        let pipeline_cache = world.resource::<PipelineCache>();
        let texture_bind_group_layout = world.resource::<TextureBindGroupLayout>();

        match self {
            Self::WaitingForImages => {
                if let Some(images) = world.get_resource::<super::Framebuffers>() {
                    let format = world.resource::<RenderAssets<Image>>()[&images[0]].texture_format;

                    let shader = world.resource::<AssetServer>().load("shaders/blur.wgsl");

                    let render_pipeline =
                        pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                            label: Some("[BlurPassPipeline] render_pipeline".into()),
                            layout: vec![
                                // @todo: need to split this into two different bindings?
                                // or make a totally separate binding for blur
                                texture_bind_group_layout.0.clone(), // unused
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
                    *self = Self::Rendering(true);
                }
            }

            Self::Rendering(_) => *self = Self::Rendering(true),
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &bevy::prelude::World,
    ) -> Result<(), render_graph::NodeRunError> {
        if !self.should_draw() {
            return Ok(());
        }

        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<Pipeline>();
        let render_pipeline = pipeline_cache
            .get_render_pipeline(pipeline.render_pipeline)
            .unwrap();

        let gpu_images = world.resource::<RenderAssets<Image>>();
        let secondary = world.resource::<super::SecondaryFramebuffer>();
        let view = &gpu_images[&secondary].texture_view;
        let bind_group = world.resource::<TextureBindGroup>();

        let mut pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("[BlurPassState] pass"),
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

        pass.set_bind_group(0, bind_group, &[]);

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
            render_app.init_resource::<TextureBindGroupLayout>();
            render_app.add_system(render_queue_texture_bind_group.in_set(RenderSet::Queue));

            let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
            // Add the compute step to the render pipeline.
            // add a node to the render graph for the simulation
            render_graph.add_node(BLUR, BlurPass::default());
            // make sure blur runs before the camera
            render_graph.add_node_edge(BLUR, bevy::render::main_graph::node::CAMERA_DRIVER);
        }
    }
}
