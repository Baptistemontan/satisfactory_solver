use crate::{
    error::{Error, Result},
    recipe::{ItemId, Recipe, RecipeId},
    solution::{Solution, SolutionResult},
};
use good_lp::{
    Expression, Solution as GLPSolution, Solver as LPSolver, SolverModel, Variable, variable,
    variables,
};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fmt::Debug,
    sync::Arc,
};

#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub iid: ItemId,
    pub qty: Option<f64>,
}

fn fast_solve_max<S: LPSolver>(
    solver: S,
    target: ItemId,
    recipes: &BTreeMap<RecipeId, Arc<Recipe>>,
    output_constraints: &BTreeMap<ItemId, f64>,
    availables: &BTreeMap<ItemId, f64>,
) -> Result<f64, S> {
    let mut vars = variables!();
    let mut recipes_var = BTreeMap::<RecipeId, Variable>::new();
    let mut items_expr = BTreeMap::new();
    for (rid, recipe) in recipes {
        let recipe_var = vars.add(variable().min(0));
        recipe.add_to_fast_solve(recipe_var, &mut items_expr);
        recipes_var.insert(*rid, recipe_var);
    }

    for (iid, qty) in output_constraints {
        if *qty != 0.0 && !items_expr.contains_key(iid) {
            return Err(Error::NoRecipe(*iid));
        }
    }

    let mut sinks = BTreeMap::new();
    for (iid, expr) in &mut items_expr {
        let min = output_constraints.get(iid).copied().unwrap_or(0.0);
        let sink_var = vars.add(variable().min(min));
        *expr -= sink_var;
        sinks.insert(*iid, sink_var);
    }

    let Some(target) = sinks.get(&target) else {
        return Err(Error::NoRecipe(target));
    };

    let mut inputs_var = BTreeMap::new();

    for (iid, max) in availables {
        let Some(expr) = items_expr.get_mut(iid) else {
            continue;
        };
        let input_var = vars.add(variable().min(0).max(*max));
        *expr += input_var;
        inputs_var.insert(*iid, input_var);
    }

    let mut problem = vars.maximise(*target).using(solver);

    for (_, expr) in items_expr {
        problem = problem.with(expr.eq(0));
    }

    let sol = match problem.solve() {
        Ok(sol) => sol,
        Err(err) => return Err(Error::SolverError(err)),
    };

    let max = sol.value(*target);

    Ok(max)
}

fn fast_solve_minimize<S: LPSolver + Clone>(
    solver: S,
    recipes: &mut BTreeMap<RecipeId, Arc<Recipe>>,
    output_constraints: &BTreeMap<ItemId, f64>,
    mut availables: BTreeMap<ItemId, f64>,
    item_cost: &BTreeMap<ItemId, f64>,
) -> SolutionResult<S> {
    let locked_outputs = find_used_recipes(
        solver.clone(),
        recipes,
        output_constraints,
        &mut availables,
        item_cost,
    )?;

    let mut vars = variables!();
    let mut recipes_var = BTreeMap::<RecipeId, Variable>::new();
    let mut items_expr = BTreeMap::new();
    for (rid, recipe) in &*recipes {
        let recipe_var = vars.add(variable().min(0));
        recipe.add_to_fast_solve(recipe_var, &mut items_expr);
        recipes_var.insert(*rid, recipe_var);
    }

    for (iid, qty) in output_constraints {
        if *qty != 0.0 && !items_expr.contains_key(iid) {
            return Err(Error::NoRecipe(*iid));
        }
    }

    let mut sinks = BTreeMap::new();
    for (iid, expr) in &mut items_expr {
        if locked_outputs.contains(iid) {
            continue;
        }
        let min = output_constraints.get(iid).copied().unwrap_or(0.0);
        let sink_var = vars.add(variable().min(min));
        *expr -= sink_var;
        sinks.insert(*iid, sink_var);
    }

    let mut inputs_var = BTreeMap::new();

    for (iid, max) in &availables {
        let Some(expr) = items_expr.get_mut(iid) else {
            continue;
        };
        let input_var = vars.add(variable().min(0).max(*max));
        *expr += input_var;
        inputs_var.insert(*iid, input_var);
    }

    let mut minimize_expr = Expression::from(0.0);

    for (iid, var) in &inputs_var {
        let cost = item_cost.get(iid).copied().unwrap_or(1.0).max(0.001);
        minimize_expr -= cost * *var;
    }

    for var in recipes_var.values() {
        minimize_expr -= *var;
    }

    let mut problem = vars.maximise(minimize_expr).using(solver);

    for (_, expr) in items_expr {
        problem = problem.with(expr.eq(0));
    }

    let sol = match problem.solve() {
        Ok(sol) => sol,
        Err(err) => return Err(Error::SolverError(err)),
    };

    let mapped_sol = Solution::from_sol_and_vars::<S>(sol, recipes_var, inputs_var, sinks);

    Ok(mapped_sol)
}

fn find_used_recipes<S: LPSolver>(
    solver: S,
    recipes: &mut BTreeMap<RecipeId, Arc<Recipe>>,
    output_constraints: &BTreeMap<ItemId, f64>,
    availables: &mut BTreeMap<ItemId, f64>,
    item_cost: &BTreeMap<ItemId, f64>,
) -> Result<BTreeSet<ItemId>, S> {
    let mut vars = variables!();
    let mut recipes_var = BTreeMap::<RecipeId, Variable>::new();
    let mut items_expr = BTreeMap::new();
    for (rid, recipe) in &*recipes {
        let recipe_var = vars.add(variable().min(0));
        recipe.add_to_fast_solve(recipe_var, &mut items_expr);
        recipes_var.insert(*rid, recipe_var);
    }

    for (iid, qty) in output_constraints {
        if *qty != 0.0 && !items_expr.contains_key(iid) {
            return Err(Error::NoRecipe(*iid));
        }
    }

    let mut sinks = BTreeMap::new();
    for (iid, expr) in &mut items_expr {
        let min = output_constraints.get(iid).copied().unwrap_or(0.0);
        let sink_var = vars.add(variable().min(min));
        *expr -= sink_var;
        sinks.insert(*iid, sink_var);
    }

    let mut inputs_var = BTreeMap::new();

    for (iid, max) in &*availables {
        let Some(expr) = items_expr.get_mut(iid) else {
            continue;
        };
        let input_var = vars.add(variable().min(0).max(*max));
        *expr += input_var;
        inputs_var.insert(*iid, input_var);
    }

    let mut minimize_expr = Expression::from(0.0);

    for (iid, var) in &inputs_var {
        let cost = item_cost.get(iid).copied().unwrap_or(1.0);
        minimize_expr -= cost * *var;
    }

    for var in recipes_var.values() {
        minimize_expr -= *var;
    }

    let mut problem = vars.maximise(minimize_expr).using(solver);

    for (_, expr) in items_expr {
        problem = problem.with(expr.eq(0));
    }

    let sol = match problem.solve() {
        Ok(sol) => sol,
        Err(err) => return Err(Error::SolverError(err)),
    };

    recipes.retain(|rid, _| {
        let Some(var) = recipes_var.get(rid) else {
            return false;
        };
        sol.value(*var) >= 0.02
    });

    availables.retain(|rid, _| {
        let Some(var) = inputs_var.get(rid) else {
            return false;
        };
        sol.value(*var) >= 0.02
    });

    let mut locked_outputs = BTreeSet::new();
    for (iid, var) in &sinks {
        if sol.value(*var) < 0.02 {
            locked_outputs.insert(*iid);
        }
    }

    Ok(locked_outputs)
}

// fn minimise_with_edges<S: LPSolver>(
//     solver: S,
//     recipes: &BTreeMap<RecipeId, Arc<Recipe>>,
//     availables: &BTreeMap<ItemId, f64>,
//     output_contraints: &BTreeMap<ItemId, f64>,
// ) -> SolutionResult<S> {
//     let mut reverse_search = BTreeMap::<ItemId, BTreeSet<RecipeId>>::new();
//     for (rid, recipe) in recipes {
//         for iid in recipe.outputs.keys() {
//             reverse_search.entry(*iid).or_default().insert(*rid);
//         }
//     }

//     let mut vars = variables!();

//     let mut recipes_vars = BTreeMap::new();
//     let mut output_expr = BTreeMap::new();
//     let mut inputs_expr = availables
//         .iter()
//         .filter(|(_, q)| **q != 0.0)
//         .map(|(iid, _)| (*iid, Expression::from(0.0)))
//         .collect();
//     let mut edge_vars = BTreeMap::new();
//     let mut in_edges_vars = BTreeMap::new();
//     let mut out_edges_vars = BTreeMap::new();
//     let mut constraints = Vec::new();

//     for (rid, recipe) in recipes {
//         let recip_var = recipes_vars
//             .entry(*rid)
//             .or_insert_with(|| vars.add(variable().min(0)));
//         recipe.add_to_solver(
//             &mut vars,
//             *rid,
//             *recip_var,
//             &mut output_expr,
//             &mut inputs_expr,
//             &reverse_search,
//             &mut edge_vars,
//             &mut in_edges_vars,
//             &mut out_edges_vars,
//             &mut constraints,
//         );
//     }

//     let mut sinks = BTreeMap::new();
//     let mut sinks_exprs = BTreeMap::new();
//     for ((_, iid), expr) in &mut output_expr {
//         let sink = sinks
//             .entry(*iid)
//             .or_insert_with(|| vars.add(variable().min(0)));
//         let sink_expr = sinks_exprs
//             .entry(*iid)
//             .or_insert_with(|| Expression::from(*sink));
//         let var = vars.add(variable().min(0));
//         *expr -= var;
//         *sink_expr -= var;
//     }

//     let mut input_vars = BTreeMap::<ItemId, Variable>::new();
//     for (iid, qty) in availables {
//         let input_var = vars.add(variable().min(0).max(*qty));
//         let Some(var) = inputs_expr.get_mut(iid) else {
//             continue;
//         };
//         *var += input_var;
//         input_vars.insert(*iid, input_var);
//     }

//     let mut minimize_expr = Expression::from(0.0);
//     for input_var in input_vars.values() {
//         minimize_expr += input_var;
//     }
//     for recipe_var in recipes_vars.values() {
//         minimize_expr += recipe_var;
//     }
//     let mut problem = vars.minimise(minimize_expr).using(solver);

//     for (iid, qty) in output_contraints {
//         let Some(sink) = sinks.get(iid) else {
//             return Err(Error::NoRecipe(*iid));
//         };
//         let constraint = constraint!(*sink == *qty);
//         problem = problem.with(constraint);
//     }

//     for constrain in constraints {
//         problem = problem.with(constrain);
//     }

//     for (_, expr) in inputs_expr {
//         problem = problem.with(expr.eq(0));
//     }

//     for (_, expr) in output_expr {
//         problem = problem.with(expr.eq(0));
//     }

//     for (_, expr) in sinks_exprs {
//         problem = problem.with(expr.eq(0));
//     }

//     let solution = match problem.solve() {
//         Ok(sol) => sol,
//         Err(err) => return Err(Error::SolverError(err)),
//     };

//     Ok(Solution::from_sol_and_vars::<S>(
//         solution,
//         recipes_vars,
//         input_vars,
//         sinks,
//         edge_vars,
//         in_edges_vars,
//         out_edges_vars,
//     ))
// }

pub fn optimize<S: LPSolver + Clone>(
    solver: S,
    targets: &[Target],
    recipes: &mut BTreeMap<RecipeId, Arc<Recipe>>,
    availables: BTreeMap<ItemId, f64>,
    item_cost: &BTreeMap<ItemId, f64>,
) -> SolutionResult<S> {
    if targets.is_empty() {
        return Err(Error::NoTarget);
    }

    let mut set_targets = targets
        .iter()
        .filter_map(|target| Some((target.iid, target.qty?)))
        .collect::<BTreeMap<ItemId, f64>>();

    let mut to_maximize = targets
        .iter()
        .filter(|target| target.qty.is_none())
        .map(|target| target.iid)
        .collect::<VecDeque<_>>();

    while let Some(target) = to_maximize.pop_front() {
        leptos::logging::log!("maximizing {:?}", target);
        let maximized = fast_solve_max(solver.clone(), target, recipes, &set_targets, &availables)?;
        leptos::logging::log!("found max: {}", maximized);
        let set_target = set_targets.entry(target).or_default();
        *set_target += maximized;
    }

    fast_solve_minimize(solver, recipes, &set_targets, availables, item_cost)
}
