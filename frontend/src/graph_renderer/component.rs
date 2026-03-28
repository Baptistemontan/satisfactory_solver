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
    available_items: RwSignal<Vec<(ItemId, AmountState)>>,
    targets: RwSignal<Vec<(ItemId, AmountState)>>,
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
        let availables = available_items.with(|available_items| {
            let mut availables = BTreeMap::new();
            for (iid, qty) in available_items {
                if let AmountState::Some(qty) = *qty {
                    availables.insert(*iid, qty);
                }
            }
            availables
        });

        let solve_for = targets.with(|targets| {
            let mut solve_for = Vec::new();
            for (iid, qty) in targets {
                let target_qty = match *qty {
                    AmountState::Maximize(_) => None,
                    AmountState::Some(qty) => Some(qty),
                    _ => continue,
                };
                solve_for.push(Target {
                    iid: *iid,
                    qty: target_qty,
                });
            }
            solve_for
        });

        let water_iid = ItemId(139);

        let item_cost = BTreeMap::from([(water_iid, 0.0)]);

        leptos::logging::log!("{:?}", solve_for);

        let solution = optimize(
            SOLVER,
            &solve_for,
            &mut solver_recipes,
            availables,
            &item_cost,
        );

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
