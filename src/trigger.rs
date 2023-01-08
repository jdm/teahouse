use bevy::prelude::*;
use crate::action::*;
use crate::message_line::StatusEvent;
use crate::player::Player;
use std::default::Default;

pub struct TriggerPlugin;

impl Plugin for TriggerPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<Triggers>()
            .add_event::<TriggerEvent>()
            .add_event::<TriggerEvent2>()
            .add_system(process_triggers)
            .add_system(process_triggers2)
            .add_startup_system(init_scripts);
    }
}

pub enum TriggerCondition {
    Manual,
    Automatic,
}

pub struct Trigger {
    pub label: String,
    pub condition: TriggerCondition,
    pub actions: Vec<Box<Action>>,
}

#[derive(Resource, Default)]
pub struct Triggers(pub Vec<Trigger>);

fn init_scripts(
    mut triggers: ResMut<Triggers>,
) {
    triggers.0.push(Trigger {
        label: "setup_vars".to_owned(),
        condition: TriggerCondition::Automatic,
        actions: vec![
            Action::SetInt(SetIntVariable {
                var: VarReference::global("times"),
                value: 0.into(),
                add_to_self: false,
            }).into(),

            Action::ManualTrigger(
                ManualTrigger::new("delay_message")
            ).into(),
        ],
    });

    triggers.0.push(Trigger {
        label: "delay_message".to_owned(),
        condition: TriggerCondition::Manual,
        actions: vec![
            Action::SetTimer(SetTimer {
                delay: 7.into(),
                trigger: "show_message".to_string(),
            }).into(),
        ],
    });

    triggers.0.push(Trigger {
        label: "show_message".to_owned(),
        condition: TriggerCondition::Manual,
        actions: vec![
            Action::SetInt(SetIntVariable {
                var: VarReference::global("times"),
                value: 1.into(),
                add_to_self: true,
            }).into(),

            Action::MessageLine(MessageLine {
                message: "This message has appeared ${times} times.".to_string(),
            }).into(),

            Action::ManualTrigger(
                ManualTrigger::new("delay_message")
            ).into(),
        ],
    });
}

pub struct TriggerEvent(pub String);

pub struct TriggerEvent2(pub String);

fn process_triggers2(
    mut triggered_events: EventReader<TriggerEvent2>,
    mut triggering_events: EventWriter<TriggerEvent>,
) {
    for event in triggered_events.iter() {
        triggering_events.send(TriggerEvent(event.0.clone()));
    }
}

fn process_triggers(
    mut triggered_events: EventReader<TriggerEvent>,
    mut triggering_events: EventWriter<TriggerEvent2>,
    mut status_events: EventWriter<StatusEvent>,
    mut scripted_timers: ResMut<ScriptedTimers>,
    mut triggers: ResMut<Triggers>,
    mut commands: Commands,
    mut variables: ResMut<VariableStorage>,
    player: Query<Entity, With<Player>>,
) {
    if player.is_empty() {
        return;
    }
    let player = player.single();

    let mut triggered = triggered_events
        .iter()
        .map(|event| event.0.clone())
        .collect::<Vec<_>>();

    let mut context = ActionContext {
        events: &mut triggering_events,
        status_events: &mut status_events,
        _commands: &mut commands,
        variables: &mut variables,
        timers: &mut scripted_timers,
        player,
    };

    let mut to_remove = vec![];
    for (idx, trigger) in triggers.0.iter().enumerate() {
        if let TriggerCondition::Automatic = trigger.condition {
            to_remove.push(idx);
            triggered.push(trigger.label.clone());
        }

        if triggered.contains(&trigger.label) {
            for action in &trigger.actions {
                action.run(&mut context);
            }
        }
    }

    for idx in to_remove.into_iter().rev() {
        triggers.0.remove(idx);
    }
}
