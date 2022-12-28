use bevy::prelude::*;
use bevy::sprite::collide_aabb::collide;
use bevy::time::FixedTimestep;

// Defines the amount of time that should elapse between each physics step.
const TIME_STEP: f32 = 1.0 / 60.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(check_for_collisions)
                .with_system(halt_collisions.after(check_for_collisions))
                .with_system(move_movable.after(halt_collisions))
        )
        .add_system(keyboard_input)
        .add_system(bevy::window::close_on_esc)
        .add_event::<CollisionEvent>()
        .run();
}

struct CollisionEvent {
    moving: Entity,
    fixed: Entity,
}

#[derive(Component)]
struct Movable {
    speed: Vec2,
    size: Vec2,
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Customer;

#[derive(Component)]
struct Prop;

fn check_for_collisions(
    q: Query<(Entity, &Movable, &Transform)>,
    timer: Res<Time>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    for (moving, movable, transform) in &q {
        for (fixed, movable2, transform2) in &q {
            if moving == fixed {
                continue;
            }

            let delta = movable.speed.extend(0.);
            let adjusted = transform.translation + delta * timer.delta_seconds();

            let collision = collide(adjusted, movable.size, transform2.translation, movable2.size);
            if collision.is_some() {
                collision_events.send(CollisionEvent {
                    moving: moving,
                    fixed: fixed,
                });
            }
        }
    }
}

fn halt_collisions(
    mut collision_events: EventReader<CollisionEvent>,
    mut q: Query<(Entity, &mut Movable)>,
) {
    for ev in collision_events.iter() {
        for (entity, mut movable) in &mut q {
            if ev.moving == entity {
                movable.speed = Vec2::ZERO;
            }
        }
    }
    collision_events.clear();
}

fn move_movable(mut q: Query<(&Movable, &mut Transform)>, timer: Res<Time>) {
    for (movable, mut transform) in &mut q {
        let delta = Vec3::new(movable.speed.x, movable.speed.y, 0.0);
        transform.translation += delta * timer.delta_seconds();
    }
}

const SPEED: f32 = 150.0;
const ENTITY_SIZE: f32 = 50.0;

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    mut q: Query<(&Player, &mut Movable)>,
) {
    let (_player, mut movable) = q.single_mut();

    if keys.just_pressed(KeyCode::Up) {
        movable.speed.y = SPEED;
    } else if keys.just_pressed(KeyCode::Down) {
        movable.speed.y = -SPEED;
    }
    if keys.any_just_released([KeyCode::Up, KeyCode::Down]) {
        movable.speed.y = 0.0;
    }

    if keys.just_pressed(KeyCode::Left) {
        movable.speed.x = -SPEED;
    } else if keys.just_pressed(KeyCode::Right) {
        movable.speed.x = SPEED;
    }
    if keys.any_just_released([KeyCode::Left, KeyCode::Right]) {
        movable.speed.x = 0.0;
    }
}

enum EntityType {
    Customer,
    Player,
}

fn spawn_sprite(entity: EntityType, color: Color, size: Vec2, position: Vec2, commands: &mut Commands) {
    let sprite = SpriteBundle {
        sprite: Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(position.x, position.y, 0.0)),
        ..default()
    };
    match entity {
        EntityType::Player => {
            commands.spawn((
                Player,
                Movable { speed: Vec2::ZERO, size },
                sprite,
            ));
        }
        EntityType::Customer => {
            commands.spawn((
                Customer,
                Movable { speed: Vec2::ZERO, size },
                sprite,
            ));
        }
    }
}

fn setup(
    mut commands: Commands,
    mut _meshes: ResMut<Assets<Mesh>>,
    mut _materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    spawn_sprite(
        EntityType::Player,
        Color::rgb(0.25, 0.25, 0.75),
        Vec2::new(ENTITY_SIZE, ENTITY_SIZE),
        Vec2::ZERO,
        &mut commands,
    );

    spawn_sprite(
        EntityType::Customer,
        Color::rgb(0.0, 0.25, 0.0),
        Vec2::new(ENTITY_SIZE, ENTITY_SIZE),
        Vec2::new(-200.0, 0.),
        &mut commands,
    );
}
