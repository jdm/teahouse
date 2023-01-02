use bevy::prelude::*;
use crate::animation::{AnimData, AnimationData, AtlasAnimationData};
use crate::customer::Customer;
use crate::entity::{Affection, RelationshipStatus, Reaction, Facing, FacingDirection};
use crate::geom::{TILE_SIZE, HasSize, MapSize, MapPos, map_to_screen};
use crate::interaction::{Interactable, PlayerInteracted};
use crate::map::Map;
use crate::message_line::{DEFAULT_EXPIRY, StatusEvent};
use crate::movable::Movable;
use crate::pathfinding::{PathfindTarget, stop_current_pathfinding};
use crate::player::Player;
use rand::Rng;
use rand::seq::IteratorRandom;
use std::time::Duration;

const CAT_SPEED: f32 = 25.0;

pub struct CatPlugin;

impl Plugin for CatPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(init_texture)
            .add_event::<SpawnCatEvent>()
            .add_system(spawn_cat)
            .add_system(interact_with_cat)
            .add_system(run_sleep)
            .add_system(run_sit)
            .add_system(run_follow)
            .add_system(run_bed);
    }
}

#[derive(Copy, Clone)]
pub enum CatAnimationState {
    WalkDown = 0,
    WalkRight = 1,
    WalkUp = 2,
    WalkLeft = 3,
    Sit = 4,
    Sleep = 5,
}

impl From<CatAnimationState> for usize {
    fn from(state: CatAnimationState) -> usize {
        state as usize
    }
}

#[derive(Component)]
pub struct CatBed;

#[derive(Component)]
pub struct Sleeping(Timer);

#[derive(Component)]
struct Sitting(Timer);

#[derive(Component)]
struct MovingToEntity;

#[derive(Component)]
struct MovingToBed;

fn run_sleep(
    mut cat: Query<(Entity, &mut State<Sleeping>, &mut AnimationData), With<Cat>>,
    humans: Query<Entity, Or<(With<Player>, With<Customer>)>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    if cat.is_empty() {
        return;
    }
    let (cat_entity, mut state, mut animation) = cat.single_mut();
    if !animation.is_current(CatAnimationState::Sleep) {
        animation.set_current(CatAnimationState::Sleep);
    }
    state.0.0.tick(time.delta());
    if state.0.0.finished() {
        commands.entity(cat_entity).remove::<State<Sleeping>>();

        let mut rng = rand::thread_rng();
        let human_entity = humans.iter().choose(&mut rng).unwrap();
        commands.entity(cat_entity).insert(PathfindTarget::new(human_entity, false));
        commands.entity(cat_entity).insert(State(MovingToEntity));
    }
}

fn run_sit(
    mut cat: Query<(Entity, &mut State<Sitting>, &mut AnimationData), With<Cat>>,
    humans: Query<Entity, Or<(With<Player>, With<Customer>)>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    if cat.is_empty() {
        return;
    }
    let (cat_entity, mut state, mut animation) = cat.single_mut();
    if !animation.is_current(CatAnimationState::Sit) {
        animation.set_current(CatAnimationState::Sit);
    }
    state.0.0.tick(time.delta());
    if state.0.0.finished() {
        commands.entity(cat_entity).remove::<State<Sitting>>();

        let mut rng = rand::thread_rng();
        let human_entity = humans.iter().choose(&mut rng).unwrap();
        commands.entity(cat_entity).insert(PathfindTarget::new(human_entity, false));
        commands.entity(cat_entity).insert(State(MovingToEntity));
    }
}

fn run_follow(
    cat: Query<(Entity, Option<&PathfindTarget>), (With<Cat>, With<State<MovingToEntity>>)>,
    cat_bed: Query<Entity, With<CatBed>>,
    mut commands: Commands,
) {
    if cat.is_empty() {
        return;
    }
    let (cat_entity, target) = cat.single();
    if target.is_none() {
        commands.entity(cat_entity).remove::<State<MovingToEntity>>();
        let cat_bed_entity = cat_bed.single();
        commands.entity(cat_entity).insert(PathfindTarget::new(cat_bed_entity, true));
        commands.entity(cat_entity).insert(State(MovingToBed));
        return;
    }

    let mut rng = rand::thread_rng();
    if rng.gen_bool(0.002) {
        let update = |entity: Entity, commands: &mut Commands| {
            commands.entity(entity).remove::<State<MovingToEntity>>();
            commands.entity(entity).insert(State(Sitting(create_sleep_timer())));
        };
        stop_current_pathfinding(cat_entity, &mut commands, update);
    }
}

fn run_bed(
    mut cat: Query<(Entity, Option<&PathfindTarget>), (With<Cat>, With<State<MovingToBed>>)>,
    mut commands: Commands,
) {
    if cat.is_empty() {
        return;
    }
    let (cat_entity, target) = cat.single_mut();
    if target.is_none() {
        commands.entity(cat_entity).remove::<State<MovingToEntity>>();
        commands.entity(cat_entity).insert(State(Sleeping(create_sleep_timer())));
    }
}

#[derive(Component, Deref, DerefMut)]
struct State<T>(pub T);

impl Default for State<Sleeping> {
    fn default() -> Self {
        Self(Sleeping(create_sleep_timer()))
    }
}

#[derive(Component, Default)]
pub struct Cat;

const MIN_SLEEP_TIME: u64 = 3;
const MAX_SLEEP_TIME: u64 = 15;

fn create_sleep_timer() -> Timer {
    let mut rng = rand::thread_rng();
    let secs = rng.gen_range(MIN_SLEEP_TIME..MAX_SLEEP_TIME);
    Timer::new(Duration::from_secs(secs), TimerMode::Once)
}

fn petting_reaction(is_sleeping: bool, affection: &Affection) -> (Reaction, String) {
    let reaction = if is_sleeping {
        Reaction::MajorNegative
    } else {
        Reaction::Positive
    };
    let message = match affection.status() {
        RelationshipStatus::Angry => "The cat hisses.",
        RelationshipStatus::Neutral => "The cat ignores you.",
        RelationshipStatus::Friendly => "The cat purrs.",
        RelationshipStatus::VeryFriendly => "The cat purrs and rubs against you.",
        RelationshipStatus::Crushing => "The cat purs and headbutts your hand.",
    }.to_string();
    (reaction, message)
}

fn interact_with_cat(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    mut cat: Query<(&mut Affection, Option<&State<Sleeping>>), With<Cat>>,
    mut status_events: EventWriter<StatusEvent>,
) {
    for event in player_interacted_events.iter() {
        let (mut affection, sleeping) = match cat.get_mut(event.interacted_entity) {
            Ok(result) => result,
            Err(_) => continue,
        };
        let (reaction, message) = petting_reaction(sleeping.is_some(), &affection);
        status_events.send(StatusEvent::timed_message(
            event.player_entity,
            message,
            DEFAULT_EXPIRY,
        ));

        affection.react(reaction);
    }
}

#[derive(Resource)]
struct CatTexture(Handle<TextureAtlas>);

fn init_texture(
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut animation_data: ResMut<AtlasAnimationData>,
    mut commands: Commands,
) {
    let texture_handle = asset_server.load("cat.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(TILE_SIZE, TILE_SIZE), 4, 5, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    animation_data.data.insert(
        texture_atlas_handle.clone(),
        vec![
            AnimData { index: 0, frames: 4, delay: 0.1, },
            AnimData { index: 4, frames: 4, delay: 0.1, },
            AnimData { index: 8, frames: 4, delay: 0.1, },
            AnimData { index: 12, frames: 4, delay: 0.1, },
            AnimData { index: 16, frames: 1, delay: 0.1, },
            AnimData { index: 17, frames: 2, delay: 0.8, },
        ],
    );
    commands.insert_resource(CatTexture(texture_atlas_handle));
}

pub struct SpawnCatEvent(pub MapPos);

fn spawn_cat(
    mut events: EventReader<SpawnCatEvent>,
    texture: Res<CatTexture>,
    mut commands: Commands,
    map: Res<Map>,
) {
    for event in events.iter() {
        let map_size = MapSize { width: 1, height: 1 };
        let rect = map_to_screen(&event.0, &map_size, &map);
        // FIXME: make better Z defaults and share them.
        let pos = Vec3::new(rect.x, rect.y, 0.9);
        let transform = Transform::from_translation(pos);

        let sprite = SpriteSheetBundle {
            texture_atlas: texture.0.clone(),
            transform,
            ..default()
        };

        let movable = Movable {
            size: Vec2::new(rect.w, rect.h),
            entity_speed: CAT_SPEED,
            ..default()
        };
        let sized = HasSize {
            size: map_size,
        };

        commands.spawn((
            Cat::default(),
            AnimationData {
                current_animation: CatAnimationState::Sleep.into(),
                facing_conversion,
            },
            Affection::default(),
            Facing(FacingDirection::Down),
            State::default(),
            Interactable {
                highlight: Color::rgb(1., 1., 1.),
                message: "Press X to pet the cat".to_string(),
                ..default()
            },
            movable,
            sized,
            sprite,
        ));
    }
}

fn facing_conversion(facing: FacingDirection) -> usize {
    match facing {
        FacingDirection::Up => CatAnimationState::WalkUp,
        FacingDirection::Down => CatAnimationState::WalkDown,
        FacingDirection::Right => CatAnimationState::WalkRight,
        FacingDirection::Left => CatAnimationState::WalkLeft,
    }.into()
}
