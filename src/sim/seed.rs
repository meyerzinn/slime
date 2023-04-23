use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
            BindingType, BufferBindingType, BufferInitDescriptor, BufferUsages, ShaderStages,
        },
        renderer::{RenderDevice, RenderQueue},
        RenderApp, RenderSet,
    },
};

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct Buffer(bevy::render::render_resource::Buffer);

impl FromWorld for Buffer {
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

fn render_prepare_update_random_seed(queue: Res<RenderQueue>, buffer: Res<Buffer>) {
    let seed: u32 = rand::random();
    queue.write_buffer(&buffer, 0, bytemuck::bytes_of(&seed));
}

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct BindGroupLayout(bevy::render::render_resource::BindGroupLayout);

impl FromWorld for BindGroupLayout {
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
pub(crate) struct BindGroup(bevy::render::render_resource::BindGroup);

impl FromWorld for BindGroup {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource();
        let layout: &BindGroupLayout = world.resource();
        let buffer: &Buffer = world.resource();
        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: "seed::BindGroup".into(),
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
        app.sub_app_mut(RenderApp)
            .init_resource::<Buffer>()
            .init_resource::<BindGroupLayout>()
            .init_resource::<BindGroup>()
            .add_system(render_prepare_update_random_seed.in_set(RenderSet::Prepare));
    }
}
