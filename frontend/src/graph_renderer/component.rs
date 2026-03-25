use std::{collections::BTreeMap, sync::Arc};

use leptos::{either::Either, prelude::*};

use super::{Graph, VisualGraph};
use core::f64;
use solver::{
    SOLVER,
    quantity::Quantity,
    recipe::{ItemId, Recipe as SolverRecipe, RecipeId},
    solver::{Solver, Target},
};

use graph::Graph as SolvedGraph;

use crate::{graph_renderer::SerializableGraph, item::AmountState, parser, recipes::Recipes};

#[component]
pub fn GraphVisualizer(
    selected_recipes: Arc<BTreeMap<RecipeId, RwSignal<bool>>>,
    available_items: Arc<BTreeMap<ItemId, RwSignal<AmountState>>>,
) -> impl IntoView {
    let recipes = expect_context::<Recipes>();

    let iron_plate_recipe_id = RecipeId(0);
    let iron_plate_item_id = ItemId(4);
    let plastic_item_id = ItemId(59);

    let iron_ingot_recipe_id = RecipeId(2);
    let iron_ore_item_id = ItemId(137);

    let target = Target {
        iid: plastic_item_id,
        qty: None,
    };

    let graph = Memo::new(move |_| {
        let mut solver_recipes = BTreeMap::new();
        for (rid, selected) in &*selected_recipes {
            if selected.get() {
                let r = recipes.get(*rid).unwrap();
                solver_recipes.insert(*rid, r.inner.clone());
            }
        }
        let mut availables = BTreeMap::new();
        for (iid, qty) in &*available_items {
            if let AmountState::Some(qty) = qty.get() {
                availables.insert(*iid, Quantity(qty));
            }
        }

        let solution = Solver::new(&solver_recipes)
            .optimize(SOLVER, &[target], &availables)
            .ok()?;

        let graph = SolvedGraph::build_from_solution(&solution, &[target.iid], &solver_recipes);

        Some(SerializableGraph::from_solved_graph(&graph))
    });

    move || match graph.get() {
        Some(graph) => Either::Left(view! {
            <Graph graph=graph/>
        }),
        None => Either::Right(view! {
            <div>"error"</div>
        }),
    }
}
