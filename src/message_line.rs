use bevy::prelude::*;
use std::time::Duration;

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

pub fn update_status_line(
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
