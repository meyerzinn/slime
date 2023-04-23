use bevy::{
    prelude::*,
    render::{
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType,
            BufferInitDescriptor, BufferUsages, ShaderStages,
        },
        renderer::RenderDevice,
        RenderApp,
    },
};

#[derive(Resource, Deref, DerefMut)]
pub struct DirectionBindGroupLayout(BindGroupLayout);

#[derive(Resource)]
pub struct DirectionBindGroups {
    pub horizontal: BindGroup,
    pub vertical: BindGroup,
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

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<DirectionBindGroupLayout>()
            .init_resource::<DirectionBindGroups>();
    }
}
