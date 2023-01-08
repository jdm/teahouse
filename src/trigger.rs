use bevy::prelude::*;
use crate::action::*;
use crate::interaction::PlayerInteracted;
use crate::message_line::StatusEvent;
use crate::player::{Player, Holding};
use crate::tea::SpawnTeapotEvent;
use std::default::Default;

pub struct TriggerPlugin;

impl Plugin for TriggerPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<Triggers>()
            .add_event::<TriggerEvent>()
            .add_event::<PlayerProximityEvent>()
            .add_system(process_triggers)
            .add_system(process_proximity)
            .add_system(process_interacted)
            .add_startup_system(init_scripts);
    }
}

pub enum TriggerCondition {
    Manual,
    Automatic,
    PlayerProximity(Entity),
    PlayerInteract(Entity),
}

pub struct Trigger {
    pub label: String,
    pub condition: TriggerCondition,
    pub actions: Vec<Box<Action>>,
}

impl Trigger {
    fn with_condition(label: String, condition: TriggerCondition) -> Trigger {
        Self {
            label,
            condition,
            actions: vec![],
        }
    }

    pub fn immediate<T: Into<String>>(label: T) -> Trigger {
        Self::with_condition(label.into(), TriggerCondition::Automatic)
    }

    pub fn player_proximity<T: Into<String>>(label: T, entity: Entity) -> Trigger {
        Self::with_condition(label.into(), TriggerCondition::PlayerProximity(entity))
    }

    pub fn player_interact<T: Into<String>>(label: T, entity: Entity) -> Trigger {
        Self::with_condition(label.into(), TriggerCondition::PlayerInteract(entity))
    }

    pub fn action(mut self, action: Action) -> Trigger {
        self.actions.push(Box::new(action));
        Trigger {
            label: self.label,
            condition: self.condition,
            actions: self.actions,
        }
    }
}

#[derive(Resource, Default)]
pub struct Triggers(pub Vec<Trigger>);

impl Triggers {
    pub fn add_trigger(&mut self, trigger: Trigger) {
        self.0.push(trigger);
    }
}

fn init_scripts(
    mut triggers: ResMut<Triggers>,
) {
    triggers.0.push(Trigger {
        label: "setup_vars".to_owned(),
        condition: TriggerCondition::Automatic,
        actions: vec![
            Action::SetInt(SetIntVariable {
                var: VarReference::global(""),
                value: 0.into(),
                add_to_self: false,
            }).into(),

            /*Action::ManualTrigger(
                ManualTrigger::new("delay_message")
            ).into(),*/
        ],
    });

    /*triggers.0.push(Trigger {
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
    });*/
}

pub struct PlayerProximityEvent(pub Entity);

fn process_proximity(
    mut proximity_events: EventReader<PlayerProximityEvent>,
    mut trigger_events: EventWriter<TriggerEvent>,
    triggers: Res<Triggers>,
) {
    for event in proximity_events.iter() {
        for trigger in &triggers.0 {
            if let TriggerCondition::PlayerProximity(entity) = trigger.condition {
                if entity == event.0 {
                    trigger_events.send(TriggerEvent(trigger.label.clone(), Some(entity)))
                }
            }
        }
    }
}

fn process_interacted(
    mut interacted_events: EventReader<PlayerInteracted>,
    mut trigger_events: EventWriter<TriggerEvent>,
    triggers: Res<Triggers>,
) {
    for event in interacted_events.iter() {
        for trigger in &triggers.0 {
            if let TriggerCondition::PlayerInteract(entity) = trigger.condition {
                if entity == event.interacted_entity {
                    trigger_events.send(TriggerEvent(trigger.label.clone(), Some(entity)))
                }
            }
        }
    }
}

pub struct TriggerEvent(pub String, pub Option<Entity>);

fn process_triggers(
    mut triggered_events: ParamSet<(
        EventReader<TriggerEvent>,
        EventWriter<TriggerEvent>,
    )>,
    mut status_events: EventWriter<StatusEvent>,
    mut spawn_teapot_events: EventWriter<SpawnTeapotEvent>,
    mut scripted_timers: ResMut<ScriptedTimers>,
    mut triggers: ResMut<Triggers>,
    mut commands: Commands,
    mut variables: ResMut<VariableStorage>,
    player: Query<(Entity, Option<&Holding>), With<Player>>,
) {
    if player.is_empty() {
        return;
    }
    let (_player_entity, player_holding) = player.single();

    let mut previous_triggered_events = triggered_events.p0();
    let mut triggered = previous_triggered_events
        .iter()
        .map(|event| (event.0.clone(), event.1.clone()))
        .collect::<Vec<_>>();

    let mut triggered_events = triggered_events.p1();
    let mut context = ActionContext {
        events: &mut triggered_events,
        status_events: &mut status_events,
        spawn_teapot_events: &mut spawn_teapot_events,
        _commands: &mut commands,
        variables: &mut variables,
        timers: &mut scripted_timers,
        triggered_entity: None,
        player_holding: player_holding.is_some(),
    };

    let mut to_remove = vec![];
    for (idx, trigger) in triggers.0.iter().enumerate() {
        context.triggered_entity = None;

        if let TriggerCondition::Automatic = trigger.condition {
            to_remove.push(idx);
            triggered.push((trigger.label.clone(), None));
        }

        let matching_trigger = triggered.iter().find(|(name, _)| *name == trigger.label);
        if let Some((_, entity)) = matching_trigger {
            context.triggered_entity = *entity;
            for action in &trigger.actions {
                action.run(&mut context);
            }
        }
    }

    for idx in to_remove.into_iter().rev() {
        triggers.0.remove(idx);
    }
}
