#![feature(array_zip)]

mod sim;

pub use sim::*;

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
    window::WindowResized,
};

const SIZE: UVec2 = UVec2::new(1536, 1536);

#[derive(Resource, Clone, Deref, DerefMut, ExtractResource)]

/// Represents the two alternating framebuffer.
pub struct Framebuffers([Handle<Image>; 2]);

#[derive(Component)]
struct FillScreen;

fn stretch_to_screen(
    resize_event: Res<Events<WindowResized>>,
    mut query: Query<&mut Sprite, With<FillScreen>>,
) {
    let mut reader = resize_event.get_reader();
    if let Some(e) = reader.iter(&resize_event).last() {
        for mut sprite in &mut query {
            sprite.custom_size = Some(Vec2 {
                x: e.width,
                y: e.height,
            });
        }
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
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(1024.0, 1024.0)),
                ..default()
            },
            texture: primary,
            ..default()
        },
        FillScreen,
    ));
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractResourcePlugin::<Framebuffers>::default())
            .add_plugin(sim::Plugin)
            .add_startup_system(setup)
            .add_system(stretch_to_screen.in_base_set(CoreSet::PreUpdate));
    }
}
