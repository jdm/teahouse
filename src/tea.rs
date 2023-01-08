use bevy::prelude::*;
use bevy::utils::Instant;
use crate::action::*;
use crate::entity::Item;
use crate::geom::{TILE_SIZE, MapSize, MapPos, HasSize, map_to_screen};
use crate::interaction::{Interactable, PlayerInteracted, AutoPickUp};
use crate::map::Map;
use crate::message_line::{DEFAULT_EXPIRY, StatusEvent};
use crate::movable::Movable;
use crate::player::Player;
use crate::trigger::{Triggers, Trigger, RunActions};
use rand::Rng;
use rand_derive2::RandGen;
use std::collections::HashMap;
use std::time::Duration;

pub struct TeaPlugin;

impl Plugin for TeaPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<SpawnTeapotEvent>()
            .add_startup_system(init_texture)
            .add_system(spawn_teapot)
            .add_system(interact_with_stash)
            .add_system(interact_with_kettles)
            .add_system(use_dirty_teapot_with_sink);
    }
}

#[derive(Component)]
pub struct Dirty;

#[derive(Component)]
pub struct Sink;

#[derive(Component)]
pub struct Kettle;

#[derive(Component)]
pub struct Cupboard {
    pub teapots: u32,
}

#[derive(Component)]
pub struct TeaStash {
    pub ingredient: Ingredient,
    pub amount: u32,
}

#[derive(Component, Default, Clone)]
pub struct TeaPot {
    pub ingredients: HashMap<Ingredient, u32>,
    pub steeped_at: Option<Instant>,
    pub steeped_for: Option<Duration>,
    pub water: u32,
}

#[derive(Hash, RandGen, Copy, Clone, PartialEq, Eq, Debug)]
pub enum Ingredient {
    BlackTea,
    OolongTea,
    Chai,
    CitrusPeel,
    MintLeaf,
    Sugar,
    Honey,
    Milk,
    Lemon,
    Rooibos,
    GreenTea,
    Matcha,
    Ceylon,
    Cream,
    BrownSugar,
}

fn use_dirty_teapot_with_sink(
    mut events: EventReader<PlayerInteracted>,
    sink: Query<Entity, With<Sink>>,
    teapots: Query<Entity, With<TeaPot>>,
    mut cupboards: Query<&mut Cupboard>,
    mut commands: Commands,
) {
    for event in events.iter() {
        let sink = sink.single();
        if event.interacted_entity != sink {
            continue;
        }
        let held = match event.held_entity {
            Some(entity) => entity,
            None => continue,
        };
        if teapots.get(held).is_ok() {
            let mut cupboard = cupboards.single_mut();
            cupboard.teapots += 1;
            commands.entity(held).despawn();
        }
    }
}

fn interact_with_stash(
    mut q: Query<&mut Player>,
    mut player_interacted_events: EventReader<PlayerInteracted>,
    mut stash: Query<&mut TeaStash>,
    mut status_events: EventWriter<StatusEvent>,
) {
    for event in player_interacted_events.iter() {
        let mut player = q.get_mut(event.player_entity).unwrap();
        let mut stash = match stash.get_mut(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };

        if stash.amount > 0 {
            stash.amount -= 1;
            let amount = player.carrying.entry(stash.ingredient).or_insert(0);
            *amount += 1;
            status_events.send(StatusEvent::timed_message(
                event.player_entity,
                format!("You take a little {:?} ({} remaining)", stash.ingredient, stash.amount),
                DEFAULT_EXPIRY,
            ));
        }
    }
}

fn interact_with_kettles(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    mut teapots: Query<&mut TeaPot>,
    mut player: Query<&mut Player>,
    kettles: Query<&Kettle>,
    mut status_events: EventWriter<StatusEvent>,
    time: Res<Time>,
) {
    for event in player_interacted_events.iter() {
        let mut player = player.get_mut(event.player_entity).unwrap();
        let _kettle = match kettles.get(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };
        let held_entity = match event.held_entity {
            Some(entity) => entity,
            None => {
                status_events.send(StatusEvent::timed_message(
                    event.player_entity,
                    "You need a teapot to use the kettle.".to_owned(),
                    DEFAULT_EXPIRY,
                ));
                continue;
            }
        };
        let mut teapot = match teapots.get_mut(held_entity) {
            Ok(teapot) => teapot,
            Err(_) => {
                status_events.send(StatusEvent::timed_message(
                    event.player_entity,
                    "You need a teapot to use the kettle.".to_owned(),
                    DEFAULT_EXPIRY,
                ));
                continue;
            }
        };

        let message = if !player.carrying.is_empty() {
            let ingredients = player
                .carrying
                .keys()
                .map(|k| format!("{:?}", k))
                .collect::<Vec<_>>();

            teapot.water = 100;
            teapot.ingredients = std::mem::take(&mut player.carrying);
            teapot.steeped_at = Some(time.last_update().unwrap());

            let ingredients = ingredients.join(" and the ");
            format!("You add the {} to the teapot and fill it with boiling water.", ingredients)
        } else {
            "You need ingredients to steep before adding the water.".to_owned()
        };

        status_events.send(StatusEvent::timed_message(
            event.player_entity,
            message,
            DEFAULT_EXPIRY,
        ));
    }
}

pub struct SpawnTeapotEvent {
    pos: MapPos,
    dirty: bool,
    pick_up: bool,
}

impl SpawnTeapotEvent {
    pub fn into_holding() -> Self {
        SpawnTeapotEvent {
            pos: MapPos { x: 0, y: 0 },
            dirty: false,
            pick_up: true,
        }
    }

    pub fn at(pos: MapPos) -> Self {
        SpawnTeapotEvent {
            pos,
            dirty: false,
            pick_up: false,
        }
    }

    #[allow(dead_code)]
    pub fn dirty(pos: MapPos) -> Self {
        SpawnTeapotEvent {
            pos,
            dirty: true,
            pick_up: false,
        }
    }
}

#[derive(Resource)]
struct TeapotTexture(Handle<Image>);

fn init_texture(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let people_handle = asset_server.load("teapot.png");
    commands.insert_resource(TeapotTexture(people_handle));
}

fn spawn_teapot(
    mut events: EventReader<SpawnTeapotEvent>,
    texture: Res<TeapotTexture>,
    mut commands: Commands,
    map: Res<Map>,
) {
    for event in events.iter() {
        let size = Vec2::new(TILE_SIZE, TILE_SIZE);
        let map_size = MapSize { width: 1, height: 1 };
        let rect = map_to_screen(&event.pos, &map_size, &map);
        // FIXME: make better Z defaults and share them.
        let pos = Vec3::new(rect.x, rect.y, 0.9);

        let sprite = SpriteBundle {
            sprite: Sprite {
                custom_size: Some(size),
                ..default()
            },
            texture: texture.0.clone(),
            transform: Transform::from_translation(pos),
            ..default()
        };

        let movable = Movable {
            size: Vec2::new(rect.w, rect.h),
            ..default()
        };
        let sized = HasSize {
            size: map_size,
        };

        let entity = commands.spawn((
            TeaPot::default(),
            Item,
            Interactable {
                message: "Press X to collect".to_string(),
                ..default()
            },
            movable,
            sized,
            sprite,
        )).id();

        if event.dirty {
            commands.entity(entity).insert(Dirty);
        }

        if event.pick_up {
            commands.entity(entity).insert(AutoPickUp);
        }
    }
}

pub fn spawn_cupboard(
    commands: &mut Commands,
    movable: Movable,
    sized: HasSize,
    transform: Transform,
    triggers: &mut Triggers,
    run_actions: &mut EventWriter<RunActions>,
) {
    let mut rng = rand::thread_rng();
    let entity = commands.spawn((
        Cupboard { teapots: rng.gen_range(4..10) },
        Interactable {
            message: "".to_string(),
            ..default()
        },
        movable,
        sized,
        transform,
    )).id();

    run_actions.send(
        Action::SetInt(SetIntVariable {
            var: VarReference::local("teapots", entity),
            value: rng.gen_range(4..10).into(),
            add_to_self: false,
        }).into()
    );

    triggers.add_trigger(
        Trigger::player_proximity("near_cupboard", entity)
            .action(Action::MessageLine(MessageLine {
                message: "Press X to pick up teapot (${self.teapots} remaining)".to_string(),
                entity: entity,
            }))
    );

    triggers.add_trigger(
        Trigger::player_interact("use_cupboard", entity)
            .action(Action::Conditional(Conditional {
                branches: vec![
                    ConditionalBranch {
                        condition: Condition::PlayerHolding,
                        actions: vec![
                            Action::MessageLine(MessageLine {
                                message: "You're already carrying something.".to_string(),
                                entity: entity,
                            }).into()
                        ],
                    },

                    ConditionalBranch {
                        condition: Condition::Int(
                            VarReference::local("teapots", entity).into(),
                            IntComparison::Equal,
                            0.into(),
                        ),
                        actions: vec![
                            Action::MessageLine(MessageLine {
                                message: "No teapots remaining.".to_string(),
                                entity: entity,
                            }).into()
                        ],
                    },
                ],
                default: vec![
                    Action::SetInt(SetIntVariable {
                        var: VarReference::local("teapots", entity).into(),
                        value: IntOrIntVar::from(-1),
                        add_to_self: true,
                    }).into(),

                    Action::SpawnHolding(SpawnHolding {
                        entity_type: Spawnable::Teapot,
                    }).into(),

                    Action::MessageLine(MessageLine {
                        message: "You take a teapot.".to_string(),
                        entity: entity,
                    }).into()
                ],
            }))
    )
}

pub fn spawn_kettle(
    commands: &mut Commands,
    movable: Movable,
    sized: HasSize,
    transform: Transform,
) {
    commands.spawn((
        Kettle,
        Interactable {
            message: "Press X to fill the pot".to_string(),
            ..default()
        },
        movable,
        sized,
        transform,
    ));
}

pub fn spawn_teastash(
    commands: &mut Commands,
    movable: Movable,
    sized: HasSize,
    transform: Transform,
    ingredient: Ingredient,
    amount: u32,
) {
    commands.spawn((
        TeaStash { ingredient, amount },
        Interactable {
            message: format!("Press X to pick up {:?}", ingredient),
            ..default()
        },
        movable,
        sized,
        transform,
    ));
}

pub fn spawn_sink(
    commands: &mut Commands,
    movable: Movable,
    sized: HasSize,
    transform: Transform,
) {
    commands.spawn((
        Sink,
        Interactable {
            message: "Press X to clean and put away pot.".to_owned(),
            ..default()
        },
        movable,
        sized,
        transform,
    ));
}
