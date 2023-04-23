use crate::Framebuffers;
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_resource::{
            AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            FilterMode, SamplerBindingType, SamplerDescriptor, ShaderStages, StorageTextureAccess,
            TextureFormat, TextureSampleType, TextureViewDimension,
        },
        renderer::RenderDevice,
        RenderApp, RenderSet,
    },
};

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct TextureBindGroupLayout(BindGroupLayout);

impl FromWorld for TextureBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource::<RenderDevice>();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "TextureBindGroupLayout".into(),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        Self(layout)
    }
}

#[derive(Resource)]
pub(crate) struct TextureBindGroups {
    pub(crate) primary: BindGroup,
    pub(crate) secondary: BindGroup,
}

fn queue_texture_bind_groups(
    mut commands: Commands,
    framebuffers: Res<Framebuffers>,
    sampler: Res<Sampler>,
    gpu_images: Res<RenderAssets<Image>>,
    layout: Res<TextureBindGroupLayout>,
    device: Res<RenderDevice>,
) {
    let [primary, secondary] = [0, 1].map(|i| {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("simulate::TextureBindGroup_{}", i)),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &gpu_images[&framebuffers[i]].texture_view,
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        })
    });
    commands.insert_resource(TextureBindGroups { primary, secondary });
}

#[derive(Resource, Deref, DerefMut)]
pub(crate) struct StorageTextureBindGroupLayout(BindGroupLayout);

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

#[derive(Resource, Deref)]
/// Stores into the primary texture.
pub(crate) struct StorageTextureBindGroup(BindGroup);

fn queue_storage_texture_bind_groups(
    mut commands: Commands,
    framebuffers: Res<Framebuffers>,
    gpu_images: Res<RenderAssets<Image>>,
    layout: Res<StorageTextureBindGroupLayout>,
    device: Res<RenderDevice>,
) {
    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: "trail::StorageTextureBindGroup".into(),
        layout: &layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(&gpu_images[&framebuffers[0]].texture_view),
        }],
    });
    commands.insert_resource(StorageTextureBindGroup(bind_group));
}

#[derive(Resource, Deref)]
struct Sampler(bevy::render::render_resource::Sampler);

impl FromWorld for Sampler {
    fn from_world(world: &mut World) -> Self {
        let device: &RenderDevice = world.resource();
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("TextureSampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        });
        Self(sampler)
    }
}

pub(crate) struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<Sampler>()
            .init_resource::<StorageTextureBindGroupLayout>()
            .init_resource::<TextureBindGroupLayout>()
            .add_system(queue_texture_bind_groups.in_set(RenderSet::Queue))
            .add_system(queue_storage_texture_bind_groups.in_set(RenderSet::Queue));
    }
}
