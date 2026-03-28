use std::{collections::BTreeMap, sync::Arc};

use good_lp::{Solution as LPSol, Solver as LPSolver, SolverModel, Variable};

use crate::{
    Fl,
    error::Result,
    recipe::{ItemId, Recipe, RecipeId},
    solver::Target,
};

pub type LPSolution<S> = <<S as LPSolver>::Model as SolverModel>::Solution;
pub type SolutionResult<S> = Result<Solution, S>;

pub struct Solution {
    pub recipes: BTreeMap<RecipeId, Fl>,
    pub inputs: BTreeMap<ItemId, Fl>,
    pub outputs: BTreeMap<ItemId, Fl>,
    // pub edges: BTreeMap<(RecipeId, RecipeId, ItemId), Fl>,
    // pub out_edges: BTreeMap<(RecipeId, ItemId), Fl>,
    // pub in_edges: BTreeMap<(ItemId, RecipeId), Fl>,
}

fn convert_and_round(value: f64) -> Fl {
    let coef = Fl::from_num(10000.0);
    let v = Fl::from_num(value) * coef;
    v.round() / coef
}

impl Solution {
    fn map<S: LPSolver, Id: Copy + Ord>(
        solution: &LPSolution<S>,
        vars: BTreeMap<Id, Variable>,
    ) -> BTreeMap<Id, Fl> {
        vars.into_iter()
            .map(|(id, var)| (id, convert_and_round(solution.value(var))))
            .collect()
    }
    pub fn from_sol_and_vars<S: LPSolver>(
        solution: LPSolution<S>,
        recipes_vars: BTreeMap<RecipeId, Variable>,
        input_vars: BTreeMap<ItemId, Variable>,
        sinks: BTreeMap<ItemId, Variable>,
        // edge_vars: BTreeMap<(RecipeId, RecipeId, ItemId), Variable>,
        // in_edges_vars: BTreeMap<(ItemId, RecipeId), Variable>,
        // out_edges_vars: BTreeMap<(RecipeId, ItemId), Variable>,
    ) -> Self {
        Self {
            recipes: Self::map::<S, _>(&solution, recipes_vars),
            inputs: Self::map::<S, _>(&solution, input_vars),
            outputs: Self::map::<S, _>(&solution, sinks),
            // edges: Self::map::<S, _>(&solution, edge_vars),
            // out_edges: Self::map::<S, _>(&solution, out_edges_vars),
            // in_edges: Self::map::<S, _>(&solution, in_edges_vars),
        }
    }
}
