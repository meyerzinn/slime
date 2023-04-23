use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{
            BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
            BindingType, BufferBindingType, BufferDescriptor, BufferUsages, ShaderStages,
        },
        renderer::{RenderDevice, RenderQueue},
        RenderApp, RenderSet,
    },
};
use bytemuck::{Pod, Zeroable};
use derive_more::From;

#[derive(Resource, From, Clone, Default, ExtractResource)]
pub struct Options {
    /// Configures how quickly trails evaporate over time. Should be in [0, 1].
    pub evaporation: f32,
    /// Lerp between trail map and blurred map.
    pub diffusion: f32,
}

#[derive(Resource, Deref)]
struct Buffer(bevy::render::render_resource::Buffer);

impl FromWorld for Buffer {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource::<RenderDevice>();
        let buffer = device.create_buffer(&BufferDescriptor {
            label: "options::Buffer".into(),
            size: std::mem::size_of::<GpuOptions>().try_into().unwrap(),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        Self(buffer)
    }
}

#[derive(Copy, Clone, Default, Pod, Zeroable)]
#[repr(C)]
struct GpuOptions {
    evaporation: f32,
    diffusion: f32,
    _padding: [f32; 2],
}

impl From<Options> for GpuOptions {
    fn from(value: Options) -> Self {
        Self {
            evaporation: value.evaporation,
            diffusion: value.diffusion,
            _padding: [0.0; 2],
        }
    }
}

fn prepare_simulation_options(queue: Res<RenderQueue>, buffer: Res<Buffer>, options: Res<Options>) {
    if options.is_changed() {
        let options = GpuOptions::from(options.clone());
        queue.write_buffer(&buffer, 0, bytemuck::bytes_of(&options))
    }
}

#[derive(Resource, Deref)]
pub(crate) struct BindGroupLayout(bevy::render::render_resource::BindGroupLayout);

impl FromWorld for BindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource::<RenderDevice>();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "OptionsBindGroupLayout".into(),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        Self(layout)
    }
}

#[derive(Resource, Deref)]
pub(crate) struct BindGroup(bevy::render::render_resource::BindGroup);

impl FromWorld for BindGroup {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource();
        let layout: &BindGroupLayout = world.resource();
        let buffer: &Buffer = world.resource();
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "options::BindGroup".into(),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Self(bind_group)
    }
}

pub(crate) struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractResourcePlugin::<Options>::default())
            // set default options
            .init_resource::<Options>();

        app.sub_app_mut(RenderApp)
            .init_resource::<BindGroupLayout>()
            .init_resource::<Buffer>()
            .init_resource::<BindGroup>()
            .add_system(prepare_simulation_options.in_set(RenderSet::Prepare));
    }
}
