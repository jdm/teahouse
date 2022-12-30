use bevy::prelude::*;
use crate::entity::*;
use crate::geom::*;
use crate::map::*;

#[derive(Component)]
pub struct DebugTile {
    pub for_entity: Entity,
}

pub fn debug_keys(
    keys: Res<Input<KeyCode>>,
    q: Query<(&Transform, &HasSize), With<Door>>,
    mut commands: Commands,
    map: Res<Map>,
) {
    if keys.just_released(KeyCode::C) {
        let (transform, sized) = q.iter().next().unwrap();
        let mut door_pos = transform_to_map_pos(&transform, &map, &sized.size);
        door_pos.x += 1;
        // FIXME: assume customers are all 1x1 entities.
        let screen_rect = map_to_screen(&door_pos, &MapSize { width: 1, height: 1 }, &map);

        let conversation = vec![
            "This is the first message.".to_string(),
            "This is the second message.".to_string(),
            "This is the third message.".to_string(),
        ];
        spawn_sprite(EntityType::Customer(conversation), screen_rect, &mut commands);
    }
}
