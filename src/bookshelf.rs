use bevy::prelude::*;
use crate::GameState;
use crate::dialog::show_message_box;
use crate::geom::HasSize;
use crate::interaction::{PlayerInteracted, Interactable};
use crate::movable::Movable;
use rand::prelude::IteratorRandom;
use rand_derive2::RandGen;

pub struct BookshelfPlugin;

impl Plugin for BookshelfPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_system(interact_with_bookshelf);
    }
}

#[derive(Component)]
struct Bookshelf {
    books: Vec<Book>,
}

#[derive(Debug)]
struct Book {
    genre: Genre,
    title: String,
}

const NOUNS: &[&str] = &[
    "Fall",
    "Time",
    "Space",
    "Mind",
    "Disease",
    "Light",
    "Sun",
    "Son",
    "Father",
    "Mother",
    "Child",
    "Dust",
    "Cowboy",
    "Professor",
    "Scientist",
    "Thief",
    "Friend",
    "Lover",
    "Man",
    "Woman",
    "Moon",
    "Baseball",
    "Food",
    "Spite",
    "Pride",
    "Ball",
    "Ship",
    "Factory",
    "Crowd",
    "Person",
    "Bird",
    "Honour",
    "Music",
    "Song",
    "Enemy",
    "Deal",
    "Plan",
    "Plant",
    "Tree",
    "Sea",
    "Ocean",
    "Depth",
    "Word",
    "Speech",
    "Touch",
    "Crown",
    "Coat",
    "King",
    "Emperor",
    "Tea",
    "Cat",
    "Dog",
    "Blame",
    "Loss",
    "Wife",
    "Husband",
    "Marriage",
    "Life",
    "Shame",
    "Day",
    "Night",
    "Empire",
    "Stadium",
    "Crowd",
];

const VERBS: &[&str] = &[
    "Dies",
    "Eats",
    "Kills",
    "Lives",
    "Survives",
    "Loves",
    "Withers",
    "Grows",
    "Wins",
    "Loses",
    "Explodes",
    "Hurts",
    "Dares",
    "Knows",
    "Recoils",
    "Sees",
    "Sings",
    "Watches",
    "Discovers",
    "Destroys",
    "Feels",
    "Thrives",
    "Flourishes",
    "Falls",
    "Rises",
];

const PATTERNS: &[&str] = &[
    "The <noun> Of <noun>",
    "The First <noun>",
    "The First <noun> <verb>",
    "The Second <noun>",
    "The Second <noun> <verb>",
    "The Final <noun>",
    "The Final <noun> <verb>",
    "When The <noun> <verb>",
    "One Last <noun>",
    "Who <verb>, <verb>",
    "From <noun> To <noun>",
    "The <noun> Gambit",
    "My <noun> <verb>",
    "Your <noun> <verb>, My <noun>",
    "The <noun> My <noun>",
    "My <noun> The <noun>",
    "Can You See The <noun>",
    "He <verb> The <noun>",
    "She <verb> The <noun>",
    "One <noun> That <verb>",
    "My Heart <verb>",
    "To Catch A <noun>",
    "The <noun> Also <verb>",
];

fn generate_title() -> String {
    let mut rng = rand::thread_rng();
    PATTERNS
        .iter()
        .choose(&mut rng)
        .unwrap()
        .replacen("<noun>", NOUNS.iter().choose(&mut rng).unwrap(), 1)
        .replacen("<noun>", NOUNS.iter().choose(&mut rng).unwrap(), 1)
        .replacen("<verb>", VERBS.iter().choose(&mut rng).unwrap(), 1)
        .replacen("<verb>", VERBS.iter().choose(&mut rng).unwrap(), 1)
}

impl Book {
    fn new() -> Book {
        Book {
            genre: Genre::generate_random(),
            title: generate_title(),
        }
    }
}

#[derive(Debug, RandGen)]
enum Genre {
    ScienceFiction,
    Romance,
    History,
    Biography,
    Sports,
}

pub fn spawn_bookshelf(
    commands: &mut Commands,
    movable: Movable,
    sized: HasSize,
    transform: Transform,
) {
    let mut books = vec![];
    for _ in 0..5 {
        books.push(Book::new());
    }
    for book in &books {
        println!("{} ({:?})", book.title, book.genre);
    }
    let bookshelf = Bookshelf {
        books,
    };
    commands.spawn((
        bookshelf,
        Interactable {
            message: "Press X to look at books".to_owned(),
            ..default()
        },
        movable,
        sized,
        transform,
    ));
}

fn interact_with_bookshelf(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    bookshelf: Query<(Entity, &Bookshelf)>,
    mut game_state: ResMut<State<GameState>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    for event in player_interacted_events.iter() {
        if let Ok((entity, bookshelf)) = bookshelf.get(event.interacted_entity) {
            let conversation = if bookshelf.books.is_empty() {
                vec!["The bookshelf is empty.".to_owned()]
            } else {
                bookshelf.books.iter()
                    .map(|book| format!("{} ({:?})", book.title, book.genre))
                    .collect()
            };
            game_state.set(GameState::Dialog).unwrap();
            show_message_box(entity, &mut commands, conversation, &asset_server);
            return;
        }
    }
}
