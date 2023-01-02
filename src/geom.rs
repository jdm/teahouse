use bevy::prelude::*;
use crate::map::Map;
use crate::movable::Movable;

pub const TILE_SIZE: f32 = 32.0;

#[derive(Component)]
pub struct HasSize {
    pub size: MapSize,
}

impl HasSize {
    pub fn screen_size(&self) -> (f32, f32) {
        (
            self.size.width as f32 * TILE_SIZE,
            self.size.height as f32 * TILE_SIZE,
        )
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct ScreenRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Default, PartialEq, Debug, Copy, Clone)]
pub struct MapPos {
    pub x: usize,
    pub y: usize,
}

#[derive(Default, PartialEq, Debug, Copy, Clone)]
pub struct MapSize {
    pub width: usize,
    pub height: usize,
}

pub fn screen_to_map_pos(x: f32, y: f32, map: &Map, size: &MapSize) -> MapPos {
    screen_to_map_pos_inner(x, y, &MapSize { width: map.width, height: map.height }, size)
}

pub fn screen_to_map_pos_inner(x: f32, y: f32, map: &MapSize, size: &MapSize) -> MapPos {
    let x = (x - size.width as f32 * TILE_SIZE / 2.) / TILE_SIZE + map.width as f32 / 2.;
    let y = -((y + size.height as f32 * TILE_SIZE / 2.) / TILE_SIZE - map.height as f32 / 2.);
    //FIXME: Sometimes trigger on web when tab is ignored for long enough.
    //       Likely too-long timesteps mean that entities can walk through walls.
    //assert!(x >= 0.);
    //assert!(y >= 0.);
    MapPos { x: x as usize, y: y as usize }
}

#[allow(dead_code)]
pub fn transform_to_screenrect(transform: &Transform, movable: &Movable) -> ScreenRect {
    ScreenRect {
        x: transform.translation.x,
        y: transform.translation.y,
        w: movable.size.x,
        h: movable.size.y,
    }
}

pub fn transform_to_map_pos(transform: &Transform, map: &Map, size: &MapSize) -> MapPos {
    let translation = transform.translation;
    screen_to_map_pos(translation.x, translation.y, map, size)
}

pub fn map_to_screen(pos: &MapPos, size: &MapSize, map: &Map) -> ScreenRect {
    let middle = ((map.width / 2) as f32, (map.height / 2) as f32);
    let screen_origin = Vec2::new(
        pos.x as f32 - middle.0,
        middle.1 as f32 - pos.y as f32,
    ) * TILE_SIZE;
    let screen_size = (
        size.width as f32 * TILE_SIZE,
        size.height as f32 * TILE_SIZE,
    );
    // Adjust from a center origin to a top-left origin.
    let origin_offset = Vec2::new(screen_size.0 / 2., -(screen_size.1 / 2.));
    let adjusted = screen_origin + origin_offset;
    ScreenRect {
        x: adjusted.x,
        y: adjusted.y,
        w: screen_size.0,
        h: screen_size.1,
    }
}

#[test]
fn coord_conversion() {
    let map = Map { width: 18, height: 8, ..default() };
    assert_eq!(map_to_screen(
        &MapPos { x: 0, y: 0 },
        &MapSize { width: 4, height: 1 },
        &map
    ), ScreenRect { x: -175., y: 87.5, w: 100., h: 25. });
}

#[test]
fn screen_to_map_subtile_x() {
    let map = Map { width: 8, height: 8, ..default() };
    let size = MapSize { width: 1, height: 1 };
    let mut screen_coords = map_to_screen(
        &MapPos { x: 4, y: 4 },
        &size,
        &map
    );
    for _ in 0..TILE_SIZE as usize {
        assert_eq!(
            screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
            MapPos { x: 4, y: 4 },
        );
        screen_coords.x += 1.;
    }

    assert_eq!(
        screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
        MapPos { x: 5, y: 4 },
    );
}

#[test]
fn screen_to_map_subtile_y() {
    let map = Map { width: 8, height: 8, ..default() };
    let size = MapSize { width: 1, height: 1 };
    let mut screen_coords = map_to_screen(
        &MapPos { x: 4, y: 4 },
        &size,
        &map
    );
    for _ in 0..TILE_SIZE as usize {
        assert_eq!(
            screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
            MapPos { x: 4, y: 4 },
        );
        screen_coords.y -= 1.;
    }

    assert_eq!(
        screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
        MapPos { x: 4, y: 5 },
    );
}

#[test]
fn screen_to_map_x_wide() {
    let map = Map { width: 8, height: 8, ..default() };
    let size = MapSize { width: 8, height: 1 };
    let screen_coords = map_to_screen(
        &MapPos { x: 4, y: 4 },
        &size,
        &map
    );
    assert_eq!(
        screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
        MapPos { x: 4, y: 4 },
    );
}

#[test]
fn screen_to_map_y_tall() {
    let map = Map { width: 8, height: 8, ..default() };
    let size = MapSize { width: 1, height: 8 };
    let screen_coords = map_to_screen(
        &MapPos { x: 4, y: 4 },
        &size,
        &map
    );
    assert_eq!(
        screen_to_map_pos(screen_coords.x, screen_coords.y, &map, &size),
        MapPos { x: 4, y: 4 },
    );
}
