use bevy::prelude::*;
use crate::customer::NewCustomerEvent;
use crate::geom::*;
use crate::map::*;
use std::default::Default;

#[derive(Resource, Default)]
pub struct DebugSettings {
    pub show_paths: bool,
}

#[derive(Component)]
pub struct DebugTile {
    pub for_entity: Entity,
}

pub fn create_debug_path(
    entity: Entity,
    path: &[MapPos],
    map: &Map,
    commands: &mut Commands,
) {
    for point in path {
        let next_screen_rect = map_to_screen(&point, &MapSize { width: 1, height: 1 }, &map);
        let next_screen_point = Vec3::new(next_screen_rect.x, next_screen_rect.y, 0.);
        commands.spawn((
            DebugTile { for_entity: entity },
            SpriteBundle {
                sprite: Sprite {
                    color: Color::rgba(0.5, 0., 0., 0.2),
                    custom_size: Some(Vec2::new(
                        next_screen_rect.w * 0.6,
                        next_screen_rect.h * 0.6,
                    )),
                    ..default()
                },
                transform: Transform::from_translation(next_screen_point),
                ..default()
            },
        ));
    }
}

pub fn debug_keys(
    keys: Res<Input<KeyCode>>,
    mut customer_events: EventWriter<NewCustomerEvent>,
    mut settings: ResMut<DebugSettings>,
) {
    if keys.just_released(KeyCode::D) {
        settings.show_paths = !settings.show_paths;
    }

    if keys.just_released(KeyCode::C) {
        customer_events.send(NewCustomerEvent);
    }
}
