use bevy::prelude::*;

#[derive(Component)]
pub struct StatusMessage {
    pub source: Option<Entity>,
}

#[derive(Bundle)]
pub struct StatusMessageBundle {
    pub message: StatusMessage,

    #[bundle]
    pub text: TextBundle,
}

pub struct StatusEvent;
