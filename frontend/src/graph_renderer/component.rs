use std::{collections::BTreeMap, sync::Arc};

use leptos::{either::Either, prelude::*};

use super::Graph;
use solver::{
    SOLVER,
    recipe::{ItemId, RecipeId},
    solver::{Target, optimize},
};

use solver::graph::Graph as SolvedGraph;

use crate::{graph_renderer::SerializableGraph, item::Items, recipes::Recipes};

#[component]
pub fn GraphVisualizer(
    selected_recipes: Arc<BTreeMap<RecipeId, RwSignal<bool>>>,
    targets: RwSignal<Vec<ItemId>>,
    availables_amount_signals: Arc<BTreeMap<ItemId, RwSignal<f64>>>,
    cost_signals: Arc<BTreeMap<ItemId, RwSignal<f64>>>,
    target_signals: Arc<BTreeMap<ItemId, RwSignal<f64>>>,
    output_maximized: Arc<BTreeMap<ItemId, RwSignal<bool>>>,
    input_enabled: Arc<BTreeMap<ItemId, RwSignal<bool>>>,
    output_enabled: Arc<BTreeMap<ItemId, RwSignal<bool>>>,
) -> impl IntoView {
    let recipes = expect_context::<Recipes>();
    let items = expect_context::<Items>();

    let graph = Memo::new(move |_| {
        let mut solver_recipes = BTreeMap::new();
        for (rid, selected) in &*selected_recipes {
            if selected.get() {
                let r = recipes.get(*rid).unwrap();
                solver_recipes.insert(*rid, r.inner.clone());
            }
        }

        let availables = items
            .items
            .keys()
            .filter_map(|iid| {
                let amount = availables_amount_signals.get(iid).unwrap().get();
                let enabled = input_enabled.get(iid).unwrap().get();

                enabled.then_some((*iid, amount))
            })
            .collect::<BTreeMap<_, _>>();

        let solve_for = targets.with(|targets| {
            targets
                .iter()
                .filter_map(|iid| {
                    let amount = target_signals.get(iid).unwrap().get();
                    let maximize = output_maximized.get(iid).unwrap().get();
                    let enabled = output_enabled.get(iid).unwrap().get();
                    let qty = (!maximize).then_some(amount);
                    (enabled && (maximize || amount > 0.0)).then_some(Target { iid: *iid, qty })
                })
                .collect::<Vec<_>>()
        });

        let item_cost = cost_signals
            .iter()
            .map(|(iid, cost)| (*iid, cost.get()))
            .collect();

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
