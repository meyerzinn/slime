#![feature(array_zip)]

mod blur;
mod sim;
pub mod species;

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
        RenderApp, RenderSet,
    },
};

const SIZE: UVec2 = UVec2::new(50, 50);

#[derive(Resource, Clone, Deref, DerefMut, ExtractResource)]
/// Represents the two alternating framebuffer.
pub struct Framebuffers([Handle<Image>; 2]);

#[derive(Resource, Clone, Deref, DerefMut, ExtractResource)]
pub struct PrimaryFramebuffer(Handle<Image>);

#[derive(Resource, Clone, Deref, DerefMut, ExtractResource)]
pub struct SecondaryFramebuffer(Handle<Image>);

// Swaps the framebuffers between render passes.
fn render_swap_framebuffers(mut commands: Commands, mut framebuffers: ResMut<Framebuffers>) {
    framebuffers.swap(0, 1);
    commands.insert_resource(PrimaryFramebuffer(framebuffers[0].clone()));
    commands.insert_resource(SecondaryFramebuffer(framebuffers[1].clone()));
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

    // spawn a sprite for each image
    for (image, visibility) in images.zip([Visibility::Hidden, Visibility::Visible]) {
        commands.spawn(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(SIZE.x as f32, SIZE.y as f32)),
                ..default()
            },
            texture: image.clone(),
            visibility,
            ..default()
        });
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup);
        app.add_plugin(ExtractResourcePlugin::<Framebuffers>::default());
        app.add_plugin(species::Plugin);
        app.add_plugin(blur::Plugin);
        app.add_plugin(sim::Plugin);

        app.sub_app_mut(RenderApp)
            .add_system(render_swap_framebuffers.in_set(RenderSet::Prepare));
    }
}
