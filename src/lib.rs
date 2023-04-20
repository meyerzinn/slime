#![feature(array_zip)]

mod sim;
pub mod species;

use bevy::{
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        RenderApp, RenderSet,
    },
};

const SIZE: UVec2 = UVec2::new(1000, 1000);

#[derive(Resource, Clone, Deref, DerefMut, ExtractResource)]
/// Represents the two alternating framebuffer.
pub struct Framebuffers([Handle<Image>; 2]);

#[derive(Resource, Clone, Deref, DerefMut)]
pub struct PrimaryFramebuffer(Handle<Image>);

impl ExtractResource for PrimaryFramebuffer {
    type Source = Framebuffers;

    fn extract_resource(source: &Self::Source) -> Self {
        PrimaryFramebuffer(source[0].clone())
    }
}

#[derive(Resource, Clone, Deref, DerefMut)]
pub struct SecondaryFramebuffer(Handle<Image>);

impl ExtractResource for SecondaryFramebuffer {
    type Source = Framebuffers;

    fn extract_resource(source: &Self::Source) -> Self {
        SecondaryFramebuffer(source[1].clone())
    }
}

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
            TextureFormat::Rgba8Unorm,
        );
        image.texture_descriptor.usage = TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::RENDER_ATTACHMENT;

        images.add(image)
    });
    commands.insert_resource(Framebuffers(images.clone()));
    let [primary, _] = images;

    // spawn a sprite for each image
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(SIZE.x as f32, SIZE.y as f32)),
            ..default()
        },
        texture: primary.clone(),
        ..default()
    });
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractResourcePlugin::<PrimaryFramebuffer>::default())
            .add_plugin(ExtractResourcePlugin::<SecondaryFramebuffer>::default())
            .add_plugin(species::Plugin)
            .add_plugin(sim::Plugin)
            .add_startup_system(setup);
    }
}
