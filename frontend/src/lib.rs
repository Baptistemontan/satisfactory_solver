use core::f64;
use std::{collections::BTreeMap, rc::Rc, sync::Arc};

use graph::Graph as SolvedGraph;
use leptos::prelude::*;

mod buildings;
mod graph_renderer;
mod item;
mod parser;
mod recipes;

use graph_renderer::Graph;
use solver::{
    SOLVER,
    quantity::Quantity,
    recipe::{ItemId, RecipeId},
    solver::{Solver, Target},
};

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
    // leptos::logging::log!("{:?}", DATA);
    let (recipes, items, buildings) = parser::parse(std::io::Cursor::new(DATA)).unwrap();

    // leptos::logging::log!("{:#?}", recipes);
    // leptos::logging::log!("{:#?}", items);
    let iron_plate_recipe_id = RecipeId(0);
    let iron_plate_item_id = ItemId(4);
    let plastic_item_id = ItemId(59);

    let iron_ingot_recipe_id = RecipeId(2);
    let iron_ore_item_id = ItemId(137);
    let crude_oil_id = ItemId(149);
    let water_id = ItemId(139);

    let solver_recipes = recipes
        .recipes
        .iter()
        .map(|(rid, recipe)| (*rid, recipe.inner.clone()))
        .collect();

    let availables = BTreeMap::from([
        (crude_oil_id, Quantity(300.0)),
        (water_id, Quantity(f64::MAX)),
    ]);

    let target = Target {
        iid: plastic_item_id,
        qty: None,
    };

    leptos::logging::log!("start solve");

    let solution = Solver::new(&solver_recipes)
        .optimize(SOLVER, &[target], &availables)
        .unwrap();

    leptos::logging::log!("finished solving");

    let graph = SolvedGraph::build_from_solution(&solution, &[target.iid], &solver_recipes);

    leptos::logging::log!("finished constructiong graph");

    provide_context(recipes);
    provide_context(items);
    provide_context(buildings);

    view! {
        <Graph graph/>
    }
}
