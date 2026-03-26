use leptos::prelude::*;

mod buildings;
mod graph_renderer;
mod item;
mod layout;
mod miner;
mod parser;
mod recipes;
mod utils;

use layout::Layout;
use thaw::{ConfigProvider, Theme};

const DATA: &str = include_str!("../../data/data.json");
const DEFAULT_BASE_URL: &str = "/";
pub const BASE_URL: &str = const {
    match option_env!("SOLVER_APP_BASE_URL") {
        Some(base_url) => base_url,
        None => DEFAULT_BASE_URL,
    }
};

#[component]
pub fn App() -> impl IntoView {
    let (recipes, items, buildings) = parser::parse(std::io::Cursor::new(DATA)).unwrap();
    provide_context(recipes);
    provide_context(items);
    provide_context(buildings);
    // let (recipes, items, buildings) = parser::parse(std::io::Cursor::new(DATA)).unwrap();
    // let iron_plate_recipe_id = RecipeId(0);
    // let iron_plate_item_id = ItemId(4);
    // let plastic_item_id = ItemId(59);

    // let iron_ingot_recipe_id = RecipeId(2);
    // let iron_ore_item_id = ItemId(137);
    // let crude_oil_id = ItemId(149);
    // let water_id = ItemId(139);

    // let solver_recipes = recipes
    //     .recipes
    //     .iter()
    //     .map(|(rid, recipe)| (*rid, recipe.inner.clone()))
    //     .collect();

    // let availables = BTreeMap::from([
    //     (crude_oil_id, Quantity(300.0)),
    //     (water_id, Quantity(f64::MAX)),
    // ]);

    // let target = Target {
    //     iid: plastic_item_id,
    //     qty: None,
    // };

    // let solution = Solver::new(&solver_recipes)
    //     .optimize(SOLVER, &[target], &availables)
    //     .unwrap();

    // let graph = SolvedGraph::build_from_solution(&solution, &[target.iid], &solver_recipes);

    // provide_context(recipes);
    // provide_context(items);
    // provide_context(buildings);

    // let visual_graph = VisualGraph::from_solved_graph(&graph);

    // view! {
    //     <Graph graph=visual_graph/>
    // }

    let theme = RwSignal::new(Theme::dark());

    view! {
        <ConfigProvider theme=theme class="thaw-provider">
            <Layout theme=theme />
        </ConfigProvider>
    }
}
