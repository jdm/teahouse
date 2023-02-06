use bevy::prelude::*;
use std::time::Duration;

pub struct MessageLinePlugin;

impl Plugin for MessageLinePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(update_status_line)
            .add_startup_system(setup)
            .add_event::<StatusEvent>();
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(
        StatusMessageBundle {
            message: StatusMessage::default(),
            text: TextBundle::from_section(
                "",
                TextStyle {
                    font: asset_server.load("Lato-Medium.ttf"),
                    font_size: 25.0,
                    color: Color::WHITE,
                },
            )
                //.with_text_alignment(TextAlignment::TOP_CENTER)
                .with_style(Style {
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        bottom: Val::Px(5.0),
                        right: Val::Px(15.0),
                        ..default()
                    },
                    ..default()
                }),
        }
    );
}

pub const DEFAULT_EXPIRY: Duration = Duration::from_secs(5);

#[derive(Component, Default)]
pub struct StatusMessage {
    pub source: Option<Entity>,
    pub timeout: Option<Timer>,
}

#[derive(Bundle)]
pub struct StatusMessageBundle {
    pub message: StatusMessage,

    #[bundle]
    pub text: TextBundle,
}

#[derive(Default)]
pub struct StatusEvent {
    message: Option<String>,
    timeout: Option<Duration>,
    source: Option<Entity>,
}

impl StatusEvent {
    #[allow(dead_code)]
    pub fn message(source: Entity, message: String) -> StatusEvent {
        StatusEvent {
            message: Some(message),
            source: Some(source),
            timeout: None,
        }
    }

    pub fn timed_message(source: Entity, message: String, duration: Duration) -> StatusEvent {
        StatusEvent {
            message: Some(message),
            source: Some(source),
            timeout: Some(duration),
        }
    }

    pub fn clear(entity: Entity) -> StatusEvent {
        StatusEvent {
            source: Some(entity),
            ..default()
        }
    }
}

fn update_status_line(
    mut status_events: EventReader<StatusEvent>,
    mut status: Query<(&mut StatusMessage, &mut Text)>,
    time: Res<Time>,
) {
    let (mut status, mut text) = status.single_mut();

    for event in status_events.iter() {
        if event.message.is_none() && event.source != status.source {
            continue;
        }
        status.source = event.source;
        status.timeout = event.timeout.map(|duration| Timer::new(duration, TimerMode::Once));
        text.sections[0].value = event.message.as_ref().cloned().unwrap_or(String::new());
    }

    let mut expired = false;
    if let Some(ref mut timer) = status.timeout {
        timer.tick(time.delta());
        expired = timer.finished();
    }

    if expired {
        status.source = None;
        status.timeout = None;
        text.sections[0].value = String::new();
    }
}
