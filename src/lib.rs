mod project;
mod sim;

use bevy::{
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            Extent3d, ShaderStages, StorageTextureAccess, Texture, TextureDimension, TextureFormat,
            TextureSampleType, TextureUsages, TextureView, TextureViewDimension,
        },
        renderer::RenderDevice,
        RenderApp, RenderSet,
    },
};

const SIZE: UVec2 = UVec2::new(1024, 1024);

#[derive(Clone, Component)]
pub struct SpeciesOptions {
    pub name: String,
    pub num_agents: u32,
}

impl ExtractComponent for SpeciesOptions {
    type Query = (Entity, &'static Self);

    type Filter = ();

    type Out = (SpeciesId, Self);

    fn extract_component(item: bevy::ecs::query::QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        let (species_id, species) = item;
        // We're going to use entity ID as species ID, since it won't change in the main world.
        // We can use species ID to cache buffers in the render world.
        Some((SpeciesId(species_id.to_bits()), species.clone()))
    }
}

#[derive(Resource, Deref, DerefMut)]
struct TexturesBindGroupLayout(BindGroupLayout);

impl FromWorld for TexturesBindGroupLayout {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: "TextureBindGroupLayout".into(),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT | ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba8Uint,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });
        Self(layout)
    }
}

#[derive(Component, Copy, Clone, Deref, DerefMut, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct SpeciesId(pub u64);

#[derive(Resource, Clone, ExtractResource, Deref, DerefMut)]
pub struct TrailImages([Handle<Image>; 2]);

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let images = [0, 1].map(|_| {
        let mut image = Image::new_fill(
            Extent3d {
                width: SIZE.x,
                height: SIZE.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Uint,
        );
        image.texture_descriptor.usage = TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::RENDER_ATTACHMENT;
        images.add(image)
    });
    commands.insert_resource(TrailImages(images));
}

#[derive(Resource, Deref, DerefMut)]
struct TrailBindGroups([BindGroup; 2]);

fn render_queue_texture_bind_group(
    mut commands: Commands,
    device: Res<RenderDevice>,
    layout: Res<TexturesBindGroupLayout>,
    gpu_images: Res<RenderAssets<Image>>,
    textures: Res<TrailImages>,
) {
    let texture_views = [0, 1].map(|i| &gpu_images[&textures[i]].texture_view);
    let bind_groups = [0, 1].map(|i| {
        device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("TrailTexturesBindGroups_{}", i)),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture_views[i]),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&texture_views[1 - i]),
                },
            ],
        })
    });
    commands.insert_resource(TrailBindGroups(bind_groups));
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
        app.add_plugin(ExtractResourcePlugin::<TrailImages>::default());
        app.add_plugin(ExtractComponentPlugin::<SpeciesOptions>::default());
        {
            let render_app = app.sub_app_mut(RenderApp);
            render_app.init_resource::<TexturesBindGroupLayout>();
            render_app.add_system(render_queue_texture_bind_group.in_set(RenderSet::Queue));
        }
        app.add_plugin(project::Plugin);
        app.add_plugin(sim::Plugin);
    }
}

/*

const SIZE: UVec2 = UVec2::new(1024, 1024);

#[derive(Resource, Clone, ExtractResource, Deref, DerefMut)]
struct SlimeTrailImages([Handle<Image>; 2]); // todo: this should probably also swap to track the SlimeTrailBindGroups?

impl FromWorld for SlimeTrailImages {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        let images = [0, 1].map(|_| {
            let mut image = Image::new_fill(
                Extent3d {
                    width: SIZE.x,
                    height: SIZE.y,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[0, 0, 0, 255],
                TextureFormat::Rgba8Unorm,
            );
            image.texture_descriptor.usage = TextureUsages::COPY_DST
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::TEXTURE_BINDING;
            images.add(image)
        });

        SlimeTrailImages(images)
    }
}

#[derive(Resource, Deref, DerefMut)]
pub struct SlimeTrailBindGroups([BindGroup; 2]);

impl FromWorld for SlimeTrailBindGroups {
    fn from_world(world: &mut World) -> Self {
        let device = world.resource::<RenderDevice>();
        let images = world.resource::<SlimeTrailImages>();
        let gpu_images = world.resource::<RenderAssets<Image>>();

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("[SlimeTrailBindGroups] layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0, // prev --> readable
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Uint,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1, // next --> writeable
                    visibility: ShaderStages::COMPUTE | ShaderStages::FRAGMENT,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::WriteOnly,
                        format: TextureFormat::Rgba8Uint,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let texture_views = [0, 1].map(|i| &gpu_images[&images[i]].texture_view);

        let bind_groups = [0, 1].map(|i| {
            device.create_bind_group(&BindGroupDescriptor {
                label: Some(&format!("[SlimeTrailBindGroups] bind_group_{}", i)),
                layout: &layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(texture_views[i]),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(texture_views[1 - i]),
                    },
                ],
            })
        });
        Self(bind_groups)
    }
}

fn swap_bind_groups(mut bind_groups: ResMut<SlimeTrailBindGroups>) {
    // at the start of every render pass, we swap the textures.
    bind_groups.swap(0, 1);
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        // app.add_startup_system(setup);
        // initialize rendering
        app.init_resource::<SlimeTrailImages>();
        {
            let render_app = app.sub_app_mut(RenderApp);
            // render_app.init_resource::<SlimeTrailBindGroups>();
            // render_app.add_system(swap_bind_groups); // at the start of every render frame, swap the bind groups
        }
        // app.add_plugin(simulation::Plugin);
    }
}
*/
