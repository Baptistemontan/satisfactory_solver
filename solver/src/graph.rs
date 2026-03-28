use std::{
    collections::{BTreeMap, VecDeque, btree_map::Entry},
    fmt::Display,
    sync::Arc,
};

use crate::{
    Fl, PRECISION,
    recipe::{ItemId, Recipe, RecipeId},
    solution::Solution,
    solver::Target,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Node {
    Recipe { rid: RecipeId, amount: Fl },
    Input { iid: ItemId, amount: Fl },
    Output { iid: ItemId, amount: Fl },
    Excess { iid: ItemId, amount: Fl },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub iid: ItemId,
    pub amount: Fl,
}

#[derive(Debug)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

fn build_graph(
    solution: &Solution,
    targets: &[Target],
    recipes: &BTreeMap<RecipeId, Arc<Recipe>>,
) -> Graph {
    let used_recipes = &solution.recipes;
    let outputs = &solution.outputs;
    let inputs = &solution.inputs;

    let mut reverse_search_recipe = BTreeMap::<ItemId, Vec<RecipeId>>::new();
    for rid in used_recipes.keys() {
        let Some(recipe) = recipes.get(rid) else {
            todo!("recipe {:#?} not found", rid);
        };
        for iid in recipe.outputs.keys() {
            reverse_search_recipe.entry(*iid).or_default().push(*rid);
        }
    }

    let mut item_queue = VecDeque::new();

    let mut nodes = Vec::new();
    for target in targets.iter() {
        let Some(target_out_qty) = outputs.get(&target.iid) else {
            continue;
        };
        item_queue.push_back((target.iid, nodes.len(), *target_out_qty));
        nodes.push(Node::Output {
            iid: target.iid,
            amount: *target_out_qty,
        });
    }

    let mut edges = Vec::<Edge>::new();
    let mut spawned_recipes = BTreeMap::<RecipeId, usize>::new();

    let mut excess = BTreeMap::<ItemId, Vec<(usize, Fl)>>::new();
    let mut input_nodes = BTreeMap::new();
    for (iid, qty) in inputs {
        input_nodes.insert(*iid, (nodes.len(), *qty));
        nodes.push(Node::Input {
            iid: *iid,
            amount: *qty,
        });
    }

    while let Some((current_item_id, needed_by, qty)) = item_queue.pop_front() {
        if let Some(recipes_to_spawn) = reverse_search_recipe.remove(&current_item_id) {
            spawn_recipes(
                &recipes_to_spawn,
                recipes,
                used_recipes,
                &mut nodes,
                &mut excess,
                &mut spawned_recipes,
                &mut item_queue,
            );
        }

        // println!("\n\ncurrent_item: {:#?}", current_item_id);
        // println!("nodes: {:#?}", nodes);
        // println!("edges: {:#?}", edges);
        // println!("input_nodes: {:#?}", input_nodes);
        // println!("excess: {:#?}", excess);

        // println!("spwaned recipes: {:#?}", spawned_recipes);

        connect_to_recipes(
            current_item_id,
            needed_by,
            qty,
            &mut edges,
            &mut excess,
            &mut input_nodes,
        );
    }

    // flush excess
    let mut possible_edges = Vec::new();
    for (excess_item_id, produced_by) in excess {
        let current_idx = nodes.len();
        let mut total_qty = Fl::ZERO;
        for (produced_by, qty) in produced_by {
            total_qty += qty;
            possible_edges.push(Edge {
                from: produced_by,
                to: current_idx,
                iid: excess_item_id,
                amount: qty,
            });
        }
        if total_qty >= 0.02 {
            edges.append(&mut possible_edges);
            nodes.push(Node::Excess {
                iid: excess_item_id,
                amount: total_qty,
            });
        } else {
            possible_edges.clear();
        }
    }

    Graph { nodes, edges }
}

fn spawn_recipes(
    recipes_ids: &[RecipeId],
    recipes: &BTreeMap<RecipeId, Arc<Recipe>>,
    used_recipes: &BTreeMap<RecipeId, Fl>,
    nodes: &mut Vec<Node>,
    excess: &mut BTreeMap<ItemId, Vec<(usize, Fl)>>,
    spawned_recipes: &mut BTreeMap<RecipeId, usize>,
    item_queue: &mut VecDeque<(ItemId, usize, Fl)>,
) {
    for rid in recipes_ids {
        if let Entry::Vacant(ve) = spawned_recipes.entry(*rid) {
            let Some(amount) = used_recipes.get(rid) else {
                todo!("recipe {:#?} amount to spawn not found", rid);
            };
            let recipe_node_idx = nodes.len();
            nodes.push(Node::Recipe {
                rid: *rid,
                amount: *amount,
            });
            ve.insert(recipe_node_idx);

            let Some(recipe) = recipes.get(rid) else {
                todo!("recipe {:#?} not found", rid);
            };

            let time_coef = Fl::from_num(60) / Fl::from_num(recipe.time);
            let per_min = time_coef * *amount;

            for (iid, qty) in &recipe.outputs {
                let out_amount = Fl::from_num(*qty) * per_min;
                excess
                    .entry(*iid)
                    .or_default()
                    .push((recipe_node_idx, out_amount));
            }
            for (iid, qty) in &recipe.inputs {
                let in_amount = Fl::from_num(*qty) * per_min;
                item_queue.push_back((*iid, recipe_node_idx, in_amount));
            }
        }
    }
}

fn connect_to_recipes(
    current_item_id: ItemId,
    needed_by: usize,
    mut qty: Fl,
    edges: &mut Vec<Edge>,
    excess: &mut BTreeMap<ItemId, Vec<(usize, Fl)>>,
    input_nodes: &mut BTreeMap<ItemId, (usize, Fl)>,
) {
    if let Entry::Occupied(mut excess) = excess.entry(current_item_id) {
        let values = excess.get_mut();
        while qty >= 0. {
            let Some((feedback_node_idx, feedback_qty)) = values.pop() else {
                excess.remove();
                break;
            };
            if (feedback_qty - qty).abs() <= PRECISION {
                if values.is_empty() {
                    excess.remove();
                }
                edges.push(Edge {
                    from: feedback_node_idx,
                    to: needed_by,
                    iid: current_item_id,
                    amount: feedback_qty,
                });
                qty = Fl::ZERO;
                break;
            } else if feedback_qty > qty {
                values.push((feedback_node_idx, feedback_qty - qty));
                edges.push(Edge {
                    from: feedback_node_idx,
                    to: needed_by,
                    iid: current_item_id,
                    amount: qty,
                });
                qty = Fl::ZERO;
                break;
            } else {
                qty -= feedback_qty;
                edges.push(Edge {
                    from: feedback_node_idx,
                    to: needed_by,
                    iid: current_item_id,
                    amount: feedback_qty,
                });
            }
        }
    }
    if qty >= 1e-5 {
        let Some((input_node_idx, input_qty)) = input_nodes.get_mut(&current_item_id) else {
            // TODO: fucking floats man ...
            return;
        };
        *input_qty -= qty;
        edges.push(Edge {
            from: *input_node_idx,
            to: needed_by,
            iid: current_item_id,
            amount: qty,
        });
    }
}

fn to_dot(f: &mut std::fmt::Formatter<'_>, nodes: &[Node], edges: &[Edge]) -> std::fmt::Result {
    writeln!(f, "digraph G {{")?;

    for (node_idx, node) in nodes.iter().enumerate() {
        match node {
            Node::Input { iid, amount } => {
                writeln!(f, "  {} [label=\"Input {:?} x{}\"];", node_idx, iid, amount)?;
            }
            Node::Output { iid, amount } | Node::Excess { iid, amount } => {
                writeln!(
                    f,
                    "  {} [label=\"Output {:?} x{}\"];",
                    node_idx, iid, amount
                )?;
            }
            Node::Recipe { rid, amount } => {
                writeln!(
                    f,
                    "  {} [label=\"Recipe {:?} x{}\"];",
                    node_idx, rid, amount
                )?;
            }
        }
    }

    for edge in edges {
        writeln!(
            f,
            "  {} -> {} [label=\"{:?} {:.1}/min\"];",
            edge.from, edge.to, edge.iid, edge.amount
        )?;
    }

    writeln!(f, "}}")
}

#[derive(Clone, Copy, Debug)]
pub struct GraphToDot<'a>(&'a Graph);

impl Graph {
    pub fn build_from_solution(
        solution: &Solution,
        targets: &[Target],
        recipes: &BTreeMap<RecipeId, Arc<Recipe>>,
    ) -> Self {
        build_graph(solution, targets, recipes)
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn to_dot(&self) -> GraphToDot<'_> {
        GraphToDot(self)
    }
}

impl Display for GraphToDot<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        to_dot(f, &self.0.nodes, &self.0.edges)
    }
}

// pub fn build_graph(
//     solution: Solution,
//     targets: &[Target],
//     recipes: &BTreeMap<RecipeId, Arc<Recipe>>,
// ) -> Graph {
//     let mut reverse_search = BTreeMap::<ItemId, Vec<(RecipeId, Fl)>>::new();
//     for (rid, recipe_amount) in &solution.recipes {
//         let recipe = recipes.get(rid).unwrap();
//         for (iid, out_amount) in &recipe.outputs {
//             let amount = *recipe_amount * Fl::from_num(*out_amount);
//             reverse_search.entry(*iid).or_default().push((*rid, amount));
//         }
//     }

//     for producers in reverse_search.values_mut() {
//         producers.sort_unstable_by_key(|a| Reverse(a.1));
//     }

//     let mut item_queue = VecDeque::new();
//     let mut item_quantities = BTreeMap::<ItemId, Fl>::new();
//     for t in targets {
//         let Some(amount) = solution.outputs.get(&t.iid) else {
//             continue;
//         };
//         item_queue.push_back(t.iid);
//         item_quantities.insert(t.iid, *amount);
//     }

//     let mut recipes_idx = BTreeMap::<RecipeId, usize>::new();
//     let mut excess = BTreeMap::<ItemId, Fl>::new();
//     let mut spawned_recipes = BTreeMap::<RecipeId, Fl>::new();
//     let mut nodes = Vec::new();

//     while let Some(current_item_id) = item_queue.pop_front() {
//         let Some(to_spawn) = reverse_search.remove(&current_item_id) else {
//             continue;
//         };

//         let mut item_amount = item_quantities.remove(&current_item_id).unwrap();
//         let item_excess = excess.remove(&current_item_id).unwrap_or_default();

//         item_amount -= item_excess;
//         for (rid, recipe_out_amount) in to_spawn {
//             if recipe_out_amount <= PRECISION {
//                 break;
//             }
//             let recipe = recipes.get(&rid).unwrap();
//             let recipe_time_coef = Fl::from_num(60) / Fl::from_num(recipe.time);
//             let recipe_out_qty = recipe.outputs.get(&current_item_id).unwrap();
//             let recipe_out_qty = Fl::from_num(*recipe_out_qty);
//             let recipe_out_per_min = recipe_time_coef * recipe_out_qty;
//             if let Some(spawned_recipe_amount) = spawned_recipes.get(&rid) {
//                 let per_min = recipe_out_per_min * *spawned_recipe_amount;
//                 item_amount -= per_min;
//             } else {
//                 let recipe_amount = if item_amount > recipe_out_amount {
//                     item_amount -= recipe_out_amount;
//                     recipe_out_amount / recipe_out_per_min
//                 } else {
//                     recipe_out_amount / item_amount
//                 };

//                 spawn_recipe(
//                     recipe,
//                     recipe_amount,
//                     &mut nodes,
//                     &mut recipes_idx,
//                     &mut item_queue,
//                     &mut item_quantities,
//                     &mut excess,
//                 );

//                 spawned_recipes.insert(rid, recipe_amount);
//             }
//         }

//         if item_amount <= PRECISION {
//             let excess = excess.entry(current_item_id).or_default();
//             *excess += item_amount;
//         } else if item_amount >= PRECISION {
//             todo!("spawn input")
//         }
//     }

//     todo!()
// }

// fn spawn_recipe(
//     recipe: &Recipe,
//     amount: Fl,
//     nodes: &mut Vec<Node>,
//     recipes_idx: &mut BTreeMap<RecipeId, usize>,
//     item_queue: &mut VecDeque<ItemId>,
//     item_quantities: &mut BTreeMap<ItemId, Fl>,
//     excess: &mut BTreeMap<ItemId, Fl>,
// ) {
// }

// #[cfg(test)]
// mod tests {
//     use std::sync::Arc;

//     use super::*;
//     use crate::{
//         SOLVER,
//         recipe::BuildingId,
//         solver::{Solver, Target},
//     };

//     #[test]
//     fn test_maximize() {
//         let available_ores = 120.;

//         let iron_ore = ItemId(0);
//         let iron_ingot = ItemId(1);
//         let iron_plate = ItemId(2);
//         let w_id = ItemId(3);
//         let iron_ingot_recipe_id = RecipeId(0);
//         let iron_plate_recipe_id = RecipeId(1);
//         let iron_ingot_recipe = Arc::new(Recipe {
//             inputs: BTreeMap::from([(iron_ore, 1.)]),
//             outputs: BTreeMap::from([(iron_ingot, 1.)]),
//             time: 2.,
//             building: BuildingId(0),
//         });
//         let iron_plate_recipe = Arc::new(Recipe {
//             inputs: BTreeMap::from([(iron_ingot, 3.)]),
//             outputs: BTreeMap::from([(iron_plate, 2.), (iron_ore, 1.), (w_id, 1.)]),
//             time: 6.,
//             building: BuildingId(0),
//         });

//         let availables = BTreeMap::from([(iron_ore, available_ores)]);
//         let recipes = BTreeMap::from([
//             (iron_ingot_recipe_id, iron_ingot_recipe),
//             (iron_plate_recipe_id, iron_plate_recipe),
//         ]);
//         let target = Target {
//             iid: iron_plate,
//             qty: None,
//         };

//         let solution = Solver::new(&recipes)
//             .optimize(SOLVER, &[target], &availables)
//             .unwrap();

//         let graph = build_graph(&recipes, &[target], &solution);

//         // println!("\n\n\n---Results---");
//         // println!("{:#?}", graph.0);
//         // println!("{:#?}", graph.1);

//         println!("\n\n\n---DOT---");
//         println!("{}", graph.to_dot());
//     }
// }
