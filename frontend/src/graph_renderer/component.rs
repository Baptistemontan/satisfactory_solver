use std::{collections::BTreeMap, sync::Arc};

use leptos::{either::Either, prelude::*};

use super::Graph;
use solver::{
    SOLVER,
    recipe::{ItemId, RecipeId},
    solver::{Target, optimize},
};

use solver::graph::Graph as SolvedGraph;

use crate::{graph_renderer::SerializableGraph, item::AmountState, recipes::Recipes};

#[component]
pub fn GraphVisualizer(
    selected_recipes: Arc<BTreeMap<RecipeId, RwSignal<bool>>>,
    available_items: Arc<BTreeMap<ItemId, RwSignal<AmountState>>>,
    targets: Arc<BTreeMap<ItemId, RwSignal<AmountState>>>,
) -> impl IntoView {
    let recipes = expect_context::<Recipes>();

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
                availables.insert(*iid, qty);
            }
        }
        let mut solve_for = Vec::new();
        for (iid, qty) in &*targets {
            let target_qty = match qty.get() {
                AmountState::Maximize(_) => None,
                AmountState::Some(qty) => Some(qty),
                _ => continue,
            };
            solve_for.push(Target {
                iid: *iid,
                qty: target_qty,
            });
        }

        leptos::logging::log!("{:?}", solve_for);

        let solution = optimize(SOLVER, &solve_for, &mut solver_recipes, availables);

        let solution = match solution {
            Ok(sol) => sol,
            Err(err) => {
                leptos::logging::log!("error: {}", err);
                return None;
            }
        };

        let graph = SolvedGraph::build_from_solution(&solution, &solve_for, &solver_recipes);

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
