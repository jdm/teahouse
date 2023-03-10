#![allow(dead_code)]

use bevy::prelude::*;
use crate::message_line::{StatusEvent, DEFAULT_EXPIRY};
use crate::tea::SpawnTeapotEvent;
use crate::trigger::TriggerEvent;
use rand::Rng;
use std::collections::HashMap;
use std::default::Default;
use std::time::Duration;

pub struct ActionPlugin;

impl Plugin for ActionPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<VariableStorage>()
            .init_resource::<ScriptedTimers>()
            .add_system(process_timers);
    }
}

#[derive(Default)]
pub struct Variables {
    pub ints: HashMap<String, i32>,
    pub strings: HashMap<String, String>,
}

impl Variables {
    fn set_int(&mut self, name: String, value: i32) {
        self.ints.insert(name, value);
    }

    fn set_string(&mut self, name: String, value: String) {
        self.strings.insert(name, value);
    }

    fn get_int(&self, name: &str) -> i32 {
        self.ints.get(name).cloned().unwrap_or(0)
    }

    fn get_string(&self, name: &str) -> String {
        self.strings.get(name).cloned().unwrap_or(String::new())
    }
}

#[derive(Resource, Default)]
pub struct VariableStorage {
    pub globals: Variables,
    pub locals: HashMap<Entity, Variables>,
}

pub struct MessageLine {
    pub message: String,
    pub entity: Entity,
}

trait InterpolatedString {
    fn eval(&self, variables: &VariableStorage, local: Option<Entity>) -> String;
}

impl InterpolatedString for String {
    fn eval(&self, variables: &VariableStorage, local: Option<Entity>) -> String {
        let mut value = self.clone();
        while let Some(start) = value.find("${") {
            if let Some(end) = value[start..].find('}') {
                let variable = &value[start..start+end+1];
                let variable_name = &variable[2..variable.len() - 1];
                let empty_variables = Variables::default();
                let (variables, variable_name) = if variable_name.starts_with("self.") {
                    let local_variables = match local {
                        Some(entity) => variables.locals.get(&entity).unwrap_or(&empty_variables),
                        None => &empty_variables,
                    };
                    (local_variables, &variable_name[5..])
                } else {
                    (&variables.globals, variable_name)
                };
                let replacement = if variables.strings.contains_key(variable_name) {
                    variables.get_string(variable_name)
                } else {
                    variables.get_int(variable_name).to_string()
                };
                value = value.replace(variable, &replacement);
            } else {
                break;
            }
        }
        value
    }
}

impl MessageLine {
    fn run(
        &self,
        variables: &VariableStorage,
        status_events: &mut EventWriter<StatusEvent>,
        local: Option<Entity>,
    ) {
        let message = self.message.eval(variables, local);
        status_events.send(StatusEvent::timed_message(self.entity, message, DEFAULT_EXPIRY));
    }
}

pub struct VarReference {
    pub name: String,
    pub local: Option<Entity>,
}

impl VarReference {
    #[allow(dead_code)]
    pub fn global<T: Into<String>>(name: T) -> Self {
        Self {
            name: name.into(),
            local: None,
        }
    }

    pub fn local<T: Into<String>>(name: T, entity: Entity) -> Self {
        Self {
            name: name.into(),
            local: Some(entity),
        }
    }
}

pub enum IntOrIntVar {
    Int(i32),
    Var(VarReference)
}

impl IntOrIntVar {
    fn eval(&self, variables: &VariableStorage) -> i32 {
        match self {
            IntOrIntVar::Int(v) => *v,
            IntOrIntVar::Var(var) => {
                let variables = if let Some(entity) = var.local {
                    variables.locals.get(&entity)
                } else {
                    Some(&variables.globals)
                };
                variables.map_or(0, |vars| vars.get_int(&var.name))
            }
        }
    }
}

impl From<i32> for IntOrIntVar {
    fn from(v: i32) -> Self {
        IntOrIntVar::Int(v)
    }
}

impl From<VarReference> for IntOrIntVar {
    fn from(var: VarReference) -> Self {
        IntOrIntVar::Var(var)
    }
}

pub enum IntOrIntVarOrRandom {
    IntOrIntVar(IntOrIntVar),
    Random {
        min: IntOrIntVar,
        max: IntOrIntVar
    },
}

impl IntOrIntVarOrRandom {
    fn eval(&self, variables: &VariableStorage) -> i32 {
        match self {
            Self::IntOrIntVar(ivar) => ivar.eval(variables),
            Self::Random { min, max } => {
                let min = min.eval(variables);
                let max = max.eval(variables);
                let mut rng = rand::thread_rng();
                rng.gen_range(min..max)
            }
        }
    }
}

impl <T: Into<IntOrIntVar>> From<T> for IntOrIntVarOrRandom {
    fn from(v: T) -> IntOrIntVarOrRandom {
        IntOrIntVarOrRandom::IntOrIntVar(v.into())
    }
}

impl From<(IntOrIntVar, IntOrIntVar)> for IntOrIntVarOrRandom {
    fn from((min, max): (IntOrIntVar, IntOrIntVar)) -> IntOrIntVarOrRandom {
        IntOrIntVarOrRandom::Random { min, max }
    }
}

pub enum StringOrStringVar {
    String(String),
    Var(VarReference)
}

impl StringOrStringVar {
    fn eval(&self, variables: &VariableStorage) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Var(var) => {
                let variables = if let Some(entity) = var.local {
                    variables.locals.get(&entity)
                } else {
                    Some(&variables.globals)
                };
                variables.map_or(String::new(), |vars| vars.get_string(&var.name))
            }
        }
    }
}

impl <T: Into<String>> From<T> for StringOrStringVar {
    fn from(s: T) -> Self {
        StringOrStringVar::String(s.into())
    }
}

impl From<VarReference> for StringOrStringVar {
    fn from(var: VarReference) -> Self {
        StringOrStringVar::Var(var)
    }
}

pub struct SetIntVariable {
    pub var: VarReference,
    pub value: IntOrIntVar,
    pub add_to_self: bool,
}

impl SetIntVariable {
    pub fn run(&self, variables: &mut VariableStorage) {
        let mut value = self.value.eval(variables);
        let variables2 = if let Some(entity) = self.var.local {
            variables.locals.entry(entity).or_insert(Variables::default())
        } else {
            &mut variables.globals
        };
        if self.add_to_self {
            value += variables2.get_int(&self.var.name);
        }
        variables2.set_int(self.var.name.clone(), value);
    }
}

pub struct SetStringVariable {
    pub var: VarReference,
    pub value: StringOrStringVar,
}

impl SetStringVariable {
    pub fn run(&self, variables: &mut VariableStorage) {
        let value = self.value.eval(variables);
        let variables = if let Some(entity) = self.var.local {
            variables.locals.entry(entity).or_insert(Variables::default())
        } else {
            &mut variables.globals
        };
        variables.set_string(self.var.name.clone(), value.clone());
    }
}

pub struct SetTimer { 
    pub delay: IntOrIntVarOrRandom,
    pub trigger: String,
}

#[derive(Default, Resource)]
pub struct ScriptedTimers(Vec<(Timer, String)>);

fn process_timers(
    mut triggering_events: EventWriter<TriggerEvent>,
    mut timers: ResMut<ScriptedTimers>,
    time: Res<Time>,
) {
    let mut to_remove = vec![];

    for (idx, timer) in timers.0.iter_mut().enumerate() {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            triggering_events.send(TriggerEvent(timer.1.clone(), None));
            to_remove.push(idx);
        }
    }

    for idx in to_remove.into_iter().rev() {
        timers.0.remove(idx);
    }
}

impl SetTimer {
    fn run(&self, variables: &VariableStorage, timers: &mut ScriptedTimers) {
        let delay = self.delay.eval(variables);
        timers.0.push((
            Timer::new(Duration::from_secs(delay as u64), TimerMode::Once),
            self.trigger.clone(),
        ));
    }
}

pub struct ManualTrigger {
    pub label: String,
}

impl ManualTrigger {
    #[allow(dead_code)]
    pub fn new<T: Into<String>>(label: T) -> Self {
        Self {
            label: label.into(),
        }
    }

    fn run(&self, events: &mut EventWriter<TriggerEvent>) {
        events.send(TriggerEvent(self.label.clone(), None));
    }
}

#[allow(dead_code)]
pub enum IntComparison {
    LessThan,
    LessThanEqual,
    Equal,
    NotEqual,
    GreaterThanEqual,
    GreaterThan,
}

pub enum Condition {
    Int(IntOrIntVar, IntComparison, IntOrIntVar),
    PlayerHolding,
}

impl Condition {
    fn eval(&self, context: &ActionContext) -> bool {
        match self {
            Condition::Int(left, op, right) => {
                let left = left.eval(context.variables);
                let right = right.eval(context.variables);
                match op {
                    IntComparison::LessThan => left < right,
                    IntComparison::LessThanEqual => left <= right,
                    IntComparison::Equal => left == right,
                    IntComparison::NotEqual => left != right,
                    IntComparison::GreaterThanEqual => left >= right,
                    IntComparison::GreaterThan => left > right,
                }
            }
            Condition::PlayerHolding => context.player_holding,
        }
    }
}

pub struct ConditionalBranch {
    pub condition: Condition,
    pub actions: Vec<Box<Action>>,
}

pub struct Conditional {
    pub branches: Vec<ConditionalBranch>,
    pub default: Vec<Box<Action>>,
}

impl Conditional {
    fn run(&self, context: &mut ActionContext) {
        for branch in &self.branches {
            if branch.condition.eval(context) {
                for action in &branch.actions {
                    action.run(context);
                }
                return;
            }
        }

        for action in &self.default {
            action.run(context);
        }
    }
}

pub enum Spawnable {
    Teapot,
}

pub struct SpawnHolding {
    pub entity_type: Spawnable,
}

impl SpawnHolding {
    fn run(&self, context: &mut ActionContext) {
        match self.entity_type {
            Spawnable::Teapot => context.spawn_teapot_events.send(SpawnTeapotEvent::into_holding()),
        }
    }
}

#[allow(dead_code)]
pub enum Action {
    SetInt(SetIntVariable),
    SetString(SetStringVariable),
    MessageLine(MessageLine),
    SetTimer(SetTimer),
    ManualTrigger(ManualTrigger),
    Conditional(Conditional),
    SpawnHolding(SpawnHolding),
}

pub struct ActionContext<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h, 'i> {
    pub events: &'a mut EventWriter<'b, 'c, TriggerEvent>,
    pub status_events: &'a mut EventWriter<'d, 'e, StatusEvent>,
    pub _commands: &'a mut Commands<'f, 'g>,
    pub spawn_teapot_events: &'a mut EventWriter<'h, 'i, SpawnTeapotEvent>,
    pub variables: &'a mut VariableStorage,
    pub timers: &'a mut ScriptedTimers,
    pub triggered_entity: Option<Entity>,
    pub player_holding: bool,
}

impl Action {
    pub fn run(&self, context: &mut ActionContext) {
        match self {
            Action::SetInt(action) => action.run(context.variables),
            Action::SetString(action) => action.run(context.variables),
            Action::MessageLine(action) => action.run(
                context.variables,
                context.status_events,
                context.triggered_entity,
            ),
            Action::SetTimer(action) => action.run(context.variables, context.timers),
            Action::ManualTrigger(action) => action.run(context.events),
            Action::Conditional(action) => action.run(context),
            Action::SpawnHolding(action) => action.run(context),
        }
    }
}
