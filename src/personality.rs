use bevy::prelude::*;
use crate::entity::{Affection, RelationshipStatus};
use rand_derive2::RandGen;
use std::collections::HashMap;
use strum::IntoEnumIterator;
use strum::EnumIter;

pub struct PersonalityPlugin;

impl Plugin for PersonalityPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(init_personalities);
    }
}

#[derive(Debug, Hash, EnumIter, RandGen, PartialEq, Eq)]
pub enum Personality {
    Frieda,
    Lucien,
    Wednesdaeigh,
    Xiaoshan,
}

/*enum Genre {
    ScienceFiction,
    Romance,
    History,
    Biography,
    Sports,
}*/

#[derive(Default)]
pub struct State {
    pub affection: Affection,
    pub likes: (),
    pub dislikes: (),
    pub visits: u32,
    pub birthday: (),
}

#[derive(Resource)]
pub struct Personalities {
    pub data: HashMap<Personality, State>
}

fn init_personalities(
    mut commands: Commands,
) {
    let mut data = HashMap::new();
    for name in Personality::iter() {
        let relationship = RelationshipStatus::generate_random();
        data.insert(name, State {
            affection: relationship.into(),
            ..default() 
        });
    }

    let personalities = Personalities {
        data,
    };
    commands.insert_resource(personalities);
}
