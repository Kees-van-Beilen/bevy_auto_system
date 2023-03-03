use bevy_auto_sys::*;
use bevy::{prelude::*, math::vec3, transform, ecs::system::EntityCommands, render::camera::RenderTarget};

fn main(){
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(MousePosition::default())
        .add_startup_system(start)
        .add_system(rotate_system)
        .add_system(should_rotate)
        .add_system(mouse_pos_system)
        .run();
}
#[derive(Component)]
pub struct Rotate(pub bool);

#[derive(Resource,Default)]
pub struct MousePosition(pub Vec2);

#[auto_system]
fn start(){
    let c = Camera2dBundle{
        projection:OrthographicProjection{
            scaling_mode:bevy::render::camera::ScalingMode::Auto { min_width: 10.0, min_height: 10.0 },
            ..Default::default()
        },
        // transform
        ..Default::default()
    };
    spawn!(c);
    let s = SpriteBundle{
        texture: load!("box.png"),
        sprite:Sprite{
            custom_size:Some(Vec2::ONE),
            ..Default::default()
        },
        ..Default::default()
    };
    spawn!(s)
        .insert(Rotate(false));
}
#[auto_system]
fn rotate_system(){
    // let dt = time!().delta_seconds();
    for (mut transform,rotate) in query![Transform Rotate] {
        if rotate.0 {
            transform.rotate_z(1.0 * (delta_time!()));
        }
    }
}
#[auto_system]
fn should_rotate(){
    for (tranform,mut rotate) in query![GlobalTransform Rotate] {
        let pos = tranform.translation().truncate();
        rotate.0 = pos.distance(res!(MousePosition).0) < 1.0;
    }
}

#[auto_system]
fn mouse_pos_system(){
    // let dt = time!().delta_seconds();
    for (camera,transform) in query![Camera GlobalTransform] {
        let window = if let RenderTarget::Window(id) = camera.target {
            windows!().get(id).unwrap()
        }else{
            windows!().get_primary().unwrap()
        };
        if let Some(world_position) = window.cursor_position()
            .and_then(|cursor| camera.viewport_to_world(transform, cursor))
            .map(|ray| ray.origin.truncate()) 
            {
                res!(mut MousePosition).0 = world_position;
        }
    }
}