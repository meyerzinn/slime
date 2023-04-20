use std::iter;

use bytemuck::{Pod, Zeroable};
use wgpu::{
    AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Color, ColorTargetState,
    ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Extent3d, FilterMode,
    FragmentState, MultisampleState, Operations, PipelineLayoutDescriptor, PrimitiveState,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor,
    SamplerBindingType, ShaderStages, StorageTextureAccess, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexState,
};
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

#[derive(Copy, Clone, Pod, Zeroable)]
#[repr(C)]
struct GpuAgent {
    position: [f32; 2],
}

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    render_pipeline: RenderPipeline,
    blur_pipeline: RenderPipeline,
    clear_pipeline: RenderPipeline,
    render_bind_groups: [BindGroup; 2],

    compute_pipeline: ComputePipeline,
    compute_bind_groups: [BindGroup; 2], // corresponding to agents
    _agents: [Texture; 2],               // chunks of 256
    slime_trail_textures: [Texture; 2],
    slime_trail_texture_views: [TextureView; 2],

    num_agents: u32,

    window: Window,
    dummy_bind_group: BindGroup,
}

impl State {
    async fn new(window: Window, num_agents: u32) -> Self {
        let simulation_resolution = 1024;
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        println!("adapters:");
        for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
            println!("- {:?}", adapter.get_info());
        }

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let device_info = adapter.get_info();
        println!("Backend: {:?}", device_info.backend);
        println!("Device Name: {}", device_info.name);
        println!("Device Type: {:?}", device_info.device_type);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.describe().srgb)
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader_module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // initialize compute pipeline
        let slime_trail_textures: [Texture; 2] = (0..2)
            .map(|i| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(&format!("slime_trail_texture_{}", i)),
                    size: wgpu::Extent3d {
                        width: simulation_resolution,
                        height: simulation_resolution,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8Unorm,
                    usage: TextureUsages::TEXTURE_BINDING
                        | TextureUsages::STORAGE_BINDING
                        | TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let slime_trail_texture_views: [TextureView; 2] = slime_trail_textures
            .iter()
            .enumerate()
            .map(|(i, slime_trail_texture)| {
                slime_trail_texture.create_view(&TextureViewDescriptor {
                    label: Some(&format!("slime_trail_texture_view_{}", i)),
                    ..TextureViewDescriptor::default()
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let create_agents = |i| {
            device.create_texture(&TextureDescriptor {
                label: Some(&format!("agents_{}", i)),
                size: Extent3d {
                    width: num_agents,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D1,
                format: TextureFormat::Rgba32Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            })
        };

        let agents: [Texture; 2] = (0..2)
            .map(create_agents)
            .collect::<Vec<Texture>>()
            .try_into()
            .unwrap();

        let compute_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("compute_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D1,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba32Float,
                            view_dimension: TextureViewDimension::D1,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
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

        let agent_views: [TextureView; 2] = agents
            .iter()
            .map(|x| x.create_view(&TextureViewDescriptor::default()))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let compute_bind_groups: [BindGroup; 2] = slime_trail_texture_views
            .iter()
            .enumerate()
            .map(|(i, slime_trail_texture_view)| {
                device.create_bind_group(&BindGroupDescriptor {
                    label: Some("compute_bind_group"),
                    layout: &compute_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: i as u32,
                            resource: BindingResource::TextureView(&agent_views[0]),
                        },
                        BindGroupEntry {
                            binding: 1 - i as u32,
                            resource: BindingResource::TextureView(&agent_views[1]),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::TextureView(&slime_trail_texture_view),
                        },
                    ],
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let dummy_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("dummy_bind_group_layout"),
            entries: &[],
        });

        let dummy_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("dummy_bind_group"),
            layout: &dummy_bind_group_layout,
            entries: &[],
        });

        let compute_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("compute_pipeline_layout"),
            bind_group_layouts: &[&dummy_bind_group_layout, &compute_bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&compute_pipeline_layout), // auto
            module: &shader_module,
            entry_point: "cs_main",
        });

        let slime_trail_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("slime_trail_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        let render_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("render_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let render_bind_groups: [BindGroup; 2] = slime_trail_texture_views
            .iter()
            .map(|texture_view| {
                device.create_bind_group(&BindGroupDescriptor {
                    label: Some("render_bind_group"),
                    layout: &render_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(texture_view),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&slime_trail_sampler),
                        },
                    ],
                })
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let primitive = PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        };

        let multisample = MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };

        let vertex = VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[],
        };

        let render_pipeline = {
            let vertex = vertex.clone();
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("render_pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex,
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::REPLACE,
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive,
                depth_stencil: None,
                multisample,
                // If the pipeline will be used with a multiview render pass, this
                // indicates how many array layers the attachments will have.
                multiview: None,
            })
        };

        // Run compute init pipeline.
        {
            let agents_init_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("agents_init_pipeline"),
                layout: Some(&compute_pipeline_layout),
                module: &shader_module,
                entry_point: "cs_init",
            });
            let mut init_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("init_encoder"),
            });
            {
                let mut init_compute_pass =
                    init_encoder.begin_compute_pass(&ComputePassDescriptor {
                        label: Some("init_compute_pass"),
                    });
                init_compute_pass.set_bind_group(0, &dummy_bind_group, &[]);
                init_compute_pass.set_bind_group(1, &compute_bind_groups[1], &[]);
                init_compute_pass.set_pipeline(&agents_init_pipeline);
                init_compute_pass.dispatch_workgroups((num_agents + 255) / 256, 1, 1);
            }
            queue.submit([init_encoder.finish()].into_iter());
        }

        let blur_pipeline = {
            let vertex = vertex.clone();
            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("blur_pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex,
                primitive,
                depth_stencil: None,
                multisample,
                fragment: Some(FragmentState {
                    module: &shader_module,
                    entry_point: "fs_blur",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: TextureFormat::Rgba8Unorm,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::REPLACE,
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
            })
        };

        let clear_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("clear_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let clear_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("clear_pipeline"),
            layout: Some(&clear_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive,
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fs_clear",
                targets: &[Some(wgpu::ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Self {
            surface,
            device,
            queue,
            size,
            config,
            render_pipeline,
            compute_pipeline,
            compute_bind_groups,
            slime_trail_textures,
            slime_trail_texture_views,

            blur_pipeline,
            clear_pipeline,

            num_agents,
            _agents: agents,
            render_bind_groups,
            dummy_bind_group,
            window,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    #[allow(unused_variables)]
    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {}

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("encoder"),
            });

        // clear the texture
        // {
        //     let mut clear_pass = encoder.begin_render_pass(&RenderPassDescriptor {
        //         label: Some("clear_render_pass"),
        //         color_attachments: &[Some(RenderPassColorAttachment {
        //             view: &self.slime_trail_texture_views[0],
        //             resolve_target: None,
        //             ops: Operations {
        //                 load: wgpu::LoadOp::Load,
        //                 store: true,
        //             },
        //         })],
        //         depth_stencil_attachment: None,
        //     });

        //     clear_pass.set_pipeline(&self.clear_pipeline);
        //     clear_pass.draw(0..4, 0..1);
        // }

        // compute pass: update agents from [0] -> [1]
        //               render trails into [0]
        {
            // todo: wtf is happening with the input & output
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute_pass"),
            });
            compute_pass.set_bind_group(0, &self.dummy_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.compute_bind_groups[0], &[]);
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.dispatch_workgroups((self.num_agents + 255) / 256, 1, 1);
        }

        // apply a blur pass from [0] -> [1]
        {
            let mut blur_render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("blur_render_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.slime_trail_texture_views[1],
                    resolve_target: None,
                    ops: Operations {
                        load: wgpu::LoadOp::Clear(Color::RED),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            blur_render_pass.set_bind_group(0, &self.render_bind_groups[0], &[]);
            blur_render_pass.set_pipeline(&self.blur_pipeline);
            blur_render_pass.draw(0..4, 0..1);
        }

        // render from [1] to framebuffer
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("draw_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });
            // render_bind_groups[1] binds the second trail texture, which is now blurred
            render_pass.set_bind_group(0, &self.render_bind_groups[1], &[]);
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..4, 0..1);
        }

        self.compute_bind_groups.swap(0, 1);
        self.render_bind_groups.swap(0, 1);
        self.slime_trail_texture_views.swap(0, 1);

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub async fn run() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // State::new uses async code, so we're going to wait for it to finish
    let mut state = State::new(window, 1000).await;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        WindowEvent::Resized(physical_size) => {
                            state.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            // new_inner_size is &mut so w have to dereference it twice
                            state.resize(**new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        state.resize(state.size)
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // We're ignoring timeouts
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                state.window().request_redraw();
            }
            _ => {}
        }
    });
}
