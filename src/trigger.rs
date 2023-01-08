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
            .add_event::<RunActions>()
            .add_system(process_triggers)
            .add_system(process_proximity)
            .add_system(process_interacted)
            .add_system(run_immediate_actions);
    }
}

#[allow(dead_code)]
pub enum TriggerCondition {
    Manual,
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
    triggers: Res<Triggers>,
    mut commands: Commands,
    mut variables: ResMut<VariableStorage>,
    player: Query<(Entity, Option<&Holding>), With<Player>>,
) {
    let mut previous_triggered_events = triggered_events.p0();
    if previous_triggered_events.is_empty() {
        return;
    }

    if player.is_empty() {
        return;
    }
    let (_player_entity, player_holding) = player.single();

    let triggered = previous_triggered_events
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

    for trigger in &triggers.0 {
        context.triggered_entity = None;

        let matching_trigger = triggered.iter().find(|(name, _)| *name == trigger.label);
        if let Some((_, entity)) = matching_trigger {
            context.triggered_entity = *entity;
            for action in &trigger.actions {
                action.run(&mut context);
            }
        }
    }
}

pub struct RunActions(pub Vec<Box<Action>>);

impl From<Action> for RunActions {
    fn from(action: Action) -> Self {
        Self(vec![action.into()])
    }
}

fn run_immediate_actions(
    mut actions: EventReader<RunActions>,
    mut trigger_events: EventWriter<TriggerEvent>,
    mut status_events: EventWriter<StatusEvent>,
    mut spawn_teapot_events: EventWriter<SpawnTeapotEvent>,
    mut scripted_timers: ResMut<ScriptedTimers>,
    mut commands: Commands,
    mut variables: ResMut<VariableStorage>,
) {
    let mut context = ActionContext {
        events: &mut trigger_events,
        status_events: &mut status_events,
        spawn_teapot_events: &mut spawn_teapot_events,
        _commands: &mut commands,
        variables: &mut variables,
        timers: &mut scripted_timers,
        triggered_entity: None,
        player_holding: false, //FIXME: should handle running before there is a player instead
    };

    for event in actions.iter() {
        for action in &event.0 {
            action.run(&mut context);
        }
    }
}
