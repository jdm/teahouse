use bevy::prelude::*;
use crate::GameState;
use crate::dialog::show_message_box;
use crate::geom::HasSize;
use crate::interaction::{PlayerInteracted, Interactable};
use crate::movable::Movable;
use crate::tea::Ingredient;
use rand::Rng;
use rand::prelude::{IteratorRandom, SliceRandom};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<StartingIngredients>()
            .add_startup_system(init_menu)
            .add_system(interact_with_menu);
    }
}

const MIN_AMOUNT: u32 = 1;
const MAX_AMOUNT: u32 = 4;

const PATTERNS: &[&str] = &[
    "Essence of <ingredient>",
    "Exotic <ingredient>",
    "Mad <ingredient>",
    "Dude, Where's My <ingredient>",
    "Whole 'Lotta <ingredient>",
    "The Quick and the <ingredient>",
    "Goin' Home To My <ingredient> in the Sky",
    "I Fell Into a Burning Ring of <ingredient>",
    "The <ingredient> and the Furious",
    "2 <ingredient> 2 Furious",
    "Give Me My <ingredient> and Nobody Gets Hurt",
    "A <ingredient>-y Hug",
    "Have <ingredient> Will Travel",
];

fn generate_name(ingredients: &[(Ingredient, u32)]) -> String {
    let mut rng = rand::thread_rng();
    let max_ingredient = ingredients
        .iter()
        .max_by_key(|(_, amount)| amount)
        .unwrap();
    PATTERNS
        .iter()
        .choose(&mut rng)
        .unwrap()
        .replace("<ingredient>", &format!("{:?}", max_ingredient.0))
}

fn generate_recipe(available: &[Ingredient], num_ingredients: usize) -> TeaRecipe {
    let mut rng = rand::thread_rng();
    let mut ingredients = vec![];
    while ingredients.len() < num_ingredients {
        let ingredient = available.choose(&mut rng).unwrap();
        let existing = ingredients.iter().map(|(i, _)| i).find(|i| **i == *ingredient);
        if existing.is_none() {
            ingredients.push((*ingredient, rng.gen_range(MIN_AMOUNT..MAX_AMOUNT)));
        }
    }
    let name = generate_name(&ingredients);
    TeaRecipe {
        ingredients,
        name,
    }
}

fn init_menu(
    mut commands: Commands,
    ingredients: Res<StartingIngredients>,
) {
    let menu = Menu {
        teas: vec![
            generate_recipe(&ingredients.ingredients, 1),
            generate_recipe(&ingredients.ingredients, 2),
            generate_recipe(&ingredients.ingredients, 3),
        ],
    };
    for tea in &menu.teas {
        println!("{:?} requires: {:?}", tea.name, tea.ingredients);
    }
    commands.insert_resource(menu);
}

#[derive(Resource)]
pub struct StartingIngredients {
    pub ingredients: Vec<Ingredient>,
}

impl FromWorld for StartingIngredients {
    fn from_world(_world: &mut World) -> Self {
        let mut ingredients = vec![];
        while ingredients.len() < 3 {
            let ingredient = Ingredient::generate_random();
            if !ingredients.contains(&ingredient) {
                ingredients.push(ingredient);
            }
        }

        StartingIngredients {
            ingredients,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TeaRecipe {
    pub ingredients: Vec<(Ingredient, u32)>,
    pub name: String,
}

#[derive(Resource, Debug)]
pub struct Menu {
    pub teas: Vec<TeaRecipe>,
}

#[derive(Component)]
pub struct MenuEntity;

fn interact_with_menu(
    mut player_interacted_events: EventReader<PlayerInteracted>,
    menu_entity: Query<Entity, With<MenuEntity>>,
    mut game_state: ResMut<State<GameState>>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    menu: Res<Menu>,
) {
    for event in player_interacted_events.iter() {
        if let Ok(menu_entity) = menu_entity.get(event.interacted_entity) {
            let conversation = menu.teas.iter()
                .map(|recipe| {
                    let mut dialogue = format!("{}\n\nIngredients:", recipe.name);
                    for (ingredient, amount) in &recipe.ingredients {
                        dialogue += &format!("\n{:?} tsp of {:?}", amount, ingredient);
                    }
                    dialogue
                })
                .collect();
            game_state.set(GameState::Dialog).unwrap();
            show_message_box(menu_entity, &mut commands, conversation, &asset_server);
            return;
        }
    }
}

pub fn spawn_menu(
    commands: &mut Commands,
    movable: Movable,
    sized: HasSize,
    transform: Transform,
) {
    commands.spawn((
        MenuEntity,
        Interactable {
            highlight: Color::rgb(1., 1., 1.),
            message: "Press X to read menu".to_owned(),
            ..default()
        },
        movable,
        sized,
        transform,
    ));
}
