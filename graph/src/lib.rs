use std::{
    collections::{BTreeMap, VecDeque, btree_map::Entry},
    fmt::Display,
    sync::Arc,
};

use float_eq::float_eq;
use good_lp::Solver as LPSolver;
use solver::{
    quantity::Quantity,
    recipe::{ItemId, Recipe, RecipeId},
    solver::Solution,
};

#[derive(Debug, Clone, Copy)]
pub enum Node {
    Recipe { rid: RecipeId, amount: f64 },
    Input { iid: ItemId, amount: f64 },
    Output { iid: ItemId, amount: f64 },
    Excess { iid: ItemId, amount: f64 },
}

impl Eq for Node {}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Recipe {
                    rid: l_rid,
                    amount: l_amount,
                },
                Self::Recipe {
                    rid: r_rid,
                    amount: r_amount,
                },
            ) => l_rid == r_rid && float_eq!(l_amount, r_amount, abs <= 1e-5),
            (
                Self::Input {
                    iid: l_iid,
                    amount: l_amount,
                },
                Self::Input {
                    iid: r_iid,
                    amount: r_amount,
                },
            ) => l_iid == r_iid && float_eq!(l_amount, r_amount, abs <= 1e-5),
            (
                Self::Output {
                    iid: l_iid,
                    amount: l_amount,
                },
                Self::Output {
                    iid: r_iid,
                    amount: r_amount,
                },
            ) => l_iid == r_iid && float_eq!(l_amount, r_amount, abs <= 1e-5),
            (
                Self::Excess {
                    iid: l_iid,
                    amount: l_amount,
                },
                Self::Excess {
                    iid: r_iid,
                    amount: r_amount,
                },
            ) => l_iid == r_iid && float_eq!(l_amount, r_amount, abs <= 1e-5),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub iid: ItemId,
    pub amount: f64,
}

impl Eq for Edge {}

impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from
            && self.to == other.to
            && self.iid == other.iid
            && float_eq!(self.amount, other.amount, abs <= 1e-5)
    }
}

#[derive(Debug)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

fn build_graph<S: LPSolver>(
    recipes: &BTreeMap<RecipeId, Arc<Recipe>>,
    targets: &[ItemId],
    solution: &Solution<S>,
) -> Graph {
    let used_recipes = solution.get_recipes();
    let outputs = solution.get_outputs();
    let inputs = solution.get_inputs();

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
        let Some(target_out_qty) = outputs.get(target) else {
            todo!("target {:#?} not found", target);
        };
        item_queue.push_back((*target, nodes.len(), *target_out_qty));
        nodes.push(Node::Output {
            iid: *target,
            amount: *target_out_qty,
        });
    }

    let mut edges = Vec::<Edge>::new();
    let mut spawned_recipes = BTreeMap::<RecipeId, usize>::new();

    let mut excess = BTreeMap::<ItemId, Vec<(usize, f64)>>::new();
    let mut input_nodes = BTreeMap::new();
    for (iid, qty) in &inputs {
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
                &used_recipes,
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
    for (excess_item_id, produced_by) in excess {
        let current_idx = nodes.len();
        let mut total_qty = 0.0;
        for (produced_by, qty) in produced_by {
            total_qty += qty;
            edges.push(Edge {
                from: produced_by,
                to: current_idx,
                iid: excess_item_id,
                amount: qty,
            });
        }
        nodes.push(Node::Excess {
            iid: excess_item_id,
            amount: total_qty,
        });
    }

    Graph { nodes, edges }
}

fn spawn_recipes(
    recipes_ids: &[RecipeId],
    recipes: &BTreeMap<RecipeId, Arc<Recipe>>,
    used_recipes: &BTreeMap<RecipeId, f64>,
    nodes: &mut Vec<Node>,
    excess: &mut BTreeMap<ItemId, Vec<(usize, f64)>>,
    spawned_recipes: &mut BTreeMap<RecipeId, usize>,
    item_queue: &mut VecDeque<(ItemId, usize, f64)>,
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

            let coef = 60. / recipe.time;

            for (iid, Quantity(qty)) in &recipe.outputs {
                excess
                    .entry(*iid)
                    .or_default()
                    .push((recipe_node_idx, *qty * *amount * coef));
            }
            for (iid, Quantity(qty)) in &recipe.inputs {
                item_queue.push_back((*iid, recipe_node_idx, *qty * *amount * coef));
            }
        }
    }
}

fn connect_to_recipes(
    current_item_id: ItemId,
    needed_by: usize,
    mut qty: f64,
    edges: &mut Vec<Edge>,
    excess: &mut BTreeMap<ItemId, Vec<(usize, f64)>>,
    input_nodes: &mut BTreeMap<ItemId, (usize, f64)>,
) {
    if let Entry::Occupied(mut excess) = excess.entry(current_item_id) {
        let values = excess.get_mut();
        while qty >= 0. {
            let Some((feedback_node_idx, feedback_qty)) = values.pop() else {
                excess.remove();
                break;
            };
            if float_eq!(feedback_qty, qty, abs <= 1e-5) {
                if values.is_empty() {
                    excess.remove();
                }
                edges.push(Edge {
                    from: feedback_node_idx,
                    to: needed_by,
                    iid: current_item_id,
                    amount: feedback_qty,
                });
                qty = 0.;
                break;
            } else if feedback_qty > qty {
                values.push((feedback_node_idx, feedback_qty - qty));
                edges.push(Edge {
                    from: feedback_node_idx,
                    to: needed_by,
                    iid: current_item_id,
                    amount: qty,
                });
                qty = 0.;
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
            todo!("?");
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
    pub fn build_from_solution<S: LPSolver>(
        solution: &Solution<S>,
        targets: &[ItemId],
        recipes: &BTreeMap<RecipeId, Arc<Recipe>>,
    ) -> Self {
        build_graph(recipes, targets, solution)
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use solver::{
        SOLVER,
        recipe::BuildingId,
        solver::{Solver, Target},
    };

    #[test]
    fn test_maximize() {
        let available_ores = 120.;

        let iron_ore = ItemId(0);
        let iron_ingot = ItemId(1);
        let iron_plate = ItemId(2);
        let w_id = ItemId(3);
        let iron_ingot_recipe_id = RecipeId(0);
        let iron_plate_recipe_id = RecipeId(1);
        let iron_ingot_recipe = Arc::new(Recipe {
            inputs: BTreeMap::from([(iron_ore, Quantity(1.))]),
            outputs: BTreeMap::from([(iron_ingot, Quantity(1.))]),
            time: 2.,
            building: BuildingId(0),
        });
        let iron_plate_recipe = Arc::new(Recipe {
            inputs: BTreeMap::from([(iron_ingot, Quantity(3.))]),
            outputs: BTreeMap::from([
                (iron_plate, Quantity(2.)),
                (iron_ore, Quantity(1.)),
                (w_id, Quantity(1.)),
            ]),
            time: 6.,
            building: BuildingId(0),
        });

        let availables = BTreeMap::from([(iron_ore, Quantity(available_ores))]);
        let recipes = BTreeMap::from([
            (iron_ingot_recipe_id, iron_ingot_recipe),
            (iron_plate_recipe_id, iron_plate_recipe),
        ]);
        let target = Target {
            iid: iron_plate,
            qty: None,
        };

        let solution = Solver::new(&recipes)
            .optimize(SOLVER, &[target], &availables)
            .unwrap();

        let graph = build_graph(&recipes, &[target.iid], &solution);

        // println!("\n\n\n---Results---");
        // println!("{:#?}", graph.0);
        // println!("{:#?}", graph.1);

        println!("\n\n\n---DOT---");
        println!("{}", graph.to_dot());
    }
}
