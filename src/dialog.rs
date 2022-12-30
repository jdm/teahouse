use bevy::prelude::*;
use crate::GameState;

#[derive(Component)]
pub struct MessageBox;

#[derive(Component)]
pub struct Conversation {
    messages: Vec<String>,
    current: usize,
    box_entity: Entity,
}

pub fn show_message_box(
    commands: &mut Commands,
    messages: Vec<String>,
    asset_server: Res<AssetServer>,
) {
    let id = commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Px(800.0), Val::Px(200.0)),
                        position_type: PositionType::Absolute,
                        position: UiRect {
                            left: Val::Px(210.0),
                            bottom: Val::Px(10.0),
                            ..default()
                        },
                        border: UiRect::all(Val::Px(20.0)),
                        ..default()
                    },
                    background_color: Color::rgb(0.4, 0.4, 1.0).into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        MessageBox,
                        TextBundle::from_sections([
                            TextSection::new(
                                &messages[0],
                                TextStyle {
                                    font: asset_server.load("Lato-Medium.ttf"),
                                    font_size: 25.0,
                                    color: Color::WHITE,
                                },
                            ),
                            TextSection::new(
                                "\n\nPress space...",
                                TextStyle {
                                    font: asset_server.load("Lato-Medium.ttf"),
                                    font_size: 15.0,
                                    color: Color::rgb(0.8, 0.8, 1.0),
                                },
                            ),
                        ]),
                    ));
                });
        })
        .id();

    commands.spawn(Conversation {
        messages,
        current: 0,
        box_entity: id,
    });
}

pub fn exit_dialog(
    conversation: Query<(Entity, &Conversation)>,
    mut commands: Commands,
) {
    let (entity, conversation) = conversation.single();
    commands.entity(conversation.box_entity).despawn_recursive();
    commands.entity(entity).despawn();
}

pub fn run_dialog(
    mut conversation: Query<&mut Conversation>,
    mut text_box: Query<&mut Text, With<MessageBox>>,
    keys: Res<Input<KeyCode>>,
    mut game_state: ResMut<State<GameState>>,
) {
    let mut conversation = conversation.single_mut();
    let mut text_box = text_box.single_mut();

    if keys.just_released(KeyCode::Space) {
        conversation.current += 1;
        if conversation.current == conversation.messages.len() {
            game_state.set(GameState::InGame).unwrap();
        } else {
            text_box.sections[0].value = conversation.messages[conversation.current].clone();
        }
    }
}
