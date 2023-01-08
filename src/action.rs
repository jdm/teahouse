use bevy::prelude::*;
use crate::message_line::{StatusEvent, DEFAULT_EXPIRY};
use crate::trigger::TriggerEvent2;
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
}

trait InterpolatedString {
    fn eval(&self, variables: &VariableStorage) -> String;
}

impl InterpolatedString for String {
    fn eval(&self, variables: &VariableStorage) -> String {
        // TODO: support local variable substitution.
        let mut value = self.clone();
        while let Some(start) = value.find("${") {
            if let Some(end) = value[start..].find('}') {
                let variable = &value[start..start+end+1];
                let variable_name = &variable[2..variable.len() - 1];
                let replacement = if variables.globals.strings.contains_key(variable_name) {
                    variables.globals.get_string(variable_name)
                } else {
                    variables.globals.get_int(variable_name).to_string()
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
        player: Entity,
    ) {
        let message = self.message.eval(variables);
        status_events.send(StatusEvent::timed_message(player, message, DEFAULT_EXPIRY));
    }
}

pub struct VarReference {
    pub name: String,
    pub local: Option<Entity>,
}

impl VarReference {
    pub fn global<T: Into<String>>(name: T) -> Self {
        Self {
            name: name.into(),
            local: None,
        }
    }

    #[allow(dead_code)]
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
    mut triggering_events: EventWriter<TriggerEvent2>,
    mut timers: ResMut<ScriptedTimers>,
    time: Res<Time>,
) {
    let mut to_remove = vec![];

    for (idx, timer) in timers.0.iter_mut().enumerate() {
        timer.0.tick(time.delta());
        if timer.0.finished() {
            triggering_events.send(TriggerEvent2(timer.1.clone()));
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
    pub fn new<T: Into<String>>(label: T) -> Self {
        Self {
            label: label.into(),
        }
    }

    fn run(&self, events: &mut EventWriter<TriggerEvent2>) {
        events.send(TriggerEvent2(self.label.clone()));
    }
}

#[allow(dead_code)]
pub enum Action {
    SetInt(SetIntVariable),
    SetString(SetStringVariable),
    MessageLine(MessageLine),
    SetTimer(SetTimer),
    ManualTrigger(ManualTrigger),
}

pub struct ActionContext<'a, 'b, 'c, 'd, 'e, 'f, 'g> {
    pub events: &'a mut EventWriter<'b, 'c, TriggerEvent2>,
    pub status_events: &'a mut EventWriter<'d, 'e, StatusEvent>,
    pub _commands: &'a mut Commands<'f, 'g>,
    pub variables: &'a mut VariableStorage,
    pub timers: &'a mut ScriptedTimers,
    pub player: Entity,
}

impl Action {
    pub fn run(&self, context: &mut ActionContext) {
        match self {
            Action::SetInt(action) => action.run(context.variables),
            Action::SetString(action) => action.run(context.variables),
            Action::MessageLine(action) => action.run(context.variables, context.status_events, context.player),
            Action::SetTimer(action) => action.run(context.variables, context.timers),
            Action::ManualTrigger(action) => action.run(context.events),
        }
    }
}
