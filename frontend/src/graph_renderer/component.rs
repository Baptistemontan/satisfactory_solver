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

use crate::{graph_renderer::SerializableGraph, parser, recipes::Recipes};

#[component]
pub fn GraphVisualizer(selected_recipes: Arc<BTreeMap<RecipeId, RwSignal<bool>>>) -> impl IntoView {
    let recipes = expect_context::<Recipes>();

    let iron_plate_recipe_id = RecipeId(0);
    let iron_plate_item_id = ItemId(4);
    let plastic_item_id = ItemId(59);

    let iron_ingot_recipe_id = RecipeId(2);
    let iron_ore_item_id = ItemId(137);
    let crude_oil_id = ItemId(149);
    let water_id = ItemId(139);

    let availables = BTreeMap::from([
        (crude_oil_id, Quantity(300.0)),
        (water_id, Quantity(f64::MAX)),
    ]);

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
