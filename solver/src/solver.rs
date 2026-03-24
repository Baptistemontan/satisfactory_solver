use crate::{
    error::{Result, try_solver_err},
    quantity::Quantity,
    recipe::{ItemId, Recipe, RecipeId},
};
use good_lp::{
    Expression, ProblemVariables, Solution as GLPSolution, Solver as LPSolver, SolverModel,
    Variable, constraint, variable, variables,
};
use std::{
    collections::{BTreeMap, VecDeque},
    fmt::Debug,
    rc::Rc,
    sync::Arc,
};

#[derive(Clone)]
pub struct Solver {
    vars: ProblemVariables,
    recipes_vars: BTreeMap<RecipeId, Variable>,
    items_exprs: BTreeMap<ItemId, Expression>,
}

#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub iid: ItemId,
    pub qty: Option<Quantity>,
}

pub type LPSolution<S> = <<S as LPSolver>::Model as SolverModel>::Solution;
pub struct Solution<S: LPSolver> {
    pub(crate) solution: LPSolution<S>,
    pub(crate) recipes_vars: BTreeMap<RecipeId, Variable>,
    pub(crate) input_vars: BTreeMap<ItemId, Variable>,
    pub(crate) sinks: BTreeMap<ItemId, Variable>,
}

pub type SolutionResult<S> = Result<Solution<S>, S>;

impl Solver {
    pub fn new(recipes: &BTreeMap<RecipeId, Arc<Recipe>>) -> Self {
        let mut vars = variables!();

        let mut recipes_vars = BTreeMap::new();
        let mut items_exprs = BTreeMap::new();

        for (rid, recipe) in recipes {
            let recipe_var = vars.add(variable().min(0));
            recipe.add_to_solver(recipe_var, &mut items_exprs);
            recipes_vars.insert(*rid, recipe_var);
        }

        Solver {
            vars,
            recipes_vars,
            items_exprs,
        }
    }

    fn solve<S: LPSolver>(
        self,
        solver: S,
        maximize_target: Option<ItemId>,
        constraints: &BTreeMap<ItemId, Quantity>,
        availables: &BTreeMap<ItemId, Quantity>,
    ) -> SolutionResult<S> {
        let Self {
            mut vars,
            mut items_exprs,
            recipes_vars,
        } = self;

        let input_vars = availables
            .iter()
            .map(|(iid, qty)| (*iid, vars.add(variable().min(0).max(*qty))))
            .collect::<BTreeMap<ItemId, Variable>>();

        let mut sinks = BTreeMap::new();

        for (iid, expr) in &mut items_exprs {
            if let Some(input_var) = input_vars.get(iid) {
                *expr += input_var;
            }
            let sink = sinks
                .entry(*iid)
                .or_insert_with(|| vars.add(variable().min(0)));
            *expr -= *sink;
        }

        let mut problem = if let Some(target_id) = maximize_target {
            let Some(target_sink) = sinks.get(&target_id) else {
                todo!("no recipe for item {:?}", target_id);
            };
            vars.maximise(target_sink).using(solver)
        } else {
            let mut inputs_expr = Expression::from(0.0);
            for input_var in input_vars.values() {
                inputs_expr += input_var;
            }
            vars.minimise(inputs_expr).using(solver)
        };

        for (iid, qty) in constraints {
            let Some(sink) = sinks.get(iid) else {
                todo!("no recipe for item {:?}", iid);
            };
            let constraint = constraint!(*sink == *qty);
            problem = problem.with(constraint)
        }

        for (_, expr) in items_exprs {
            problem = problem.with(expr.eq(0));
        }

        let solution = try_solver_err!(problem.solve());

        Ok(Solution {
            solution,
            input_vars,
            sinks,
            recipes_vars,
        })
    }

    pub fn optimize<S: LPSolver + Clone>(
        self,
        solver: S,
        targets: &[Target],
        availables: &BTreeMap<ItemId, Quantity>,
    ) -> SolutionResult<S> {
        let mut set_targets = targets
            .iter()
            .filter_map(|target| Some((target.iid, target.qty?)))
            .collect::<BTreeMap<ItemId, Quantity>>();

        let mut to_maximize = targets
            .iter()
            .filter(|target| target.qty.is_none())
            .map(|target| target.iid)
            .collect::<VecDeque<_>>();

        while let Some(target) = to_maximize.pop_front() {
            let solution =
                self.clone()
                    .solve(solver.clone(), Some(target), &set_targets, availables)?;
            let Some(target_sink) = solution.sinks.get(&target) else {
                todo!("target {:#?} sink not found", target);
            };
            let maximized = solution.solution.value(*target_sink);
            let set_target = set_targets.entry(target).or_default();
            *set_target += maximized;
        }

        self.solve(solver, None, &set_targets, availables)
    }
}

impl<S: LPSolver> Solution<S> {
    pub fn get_inputs(&self) -> BTreeMap<ItemId, f64> {
        self.input_vars
            .iter()
            .map(|(iid, var)| {
                let qty = self.solution.value(*var);
                (*iid, qty)
            })
            .filter(|(_, qty)| qty.abs() >= 1e-5)
            .collect()
    }

    pub fn get_outputs(&self) -> BTreeMap<ItemId, f64> {
        self.sinks
            .iter()
            .map(|(iid, var)| {
                let qty = self.solution.value(*var);
                (*iid, qty)
            })
            .filter(|(_, qty)| qty.abs() >= 1e-5)
            .collect()
    }

    pub fn get_recipes(&self) -> BTreeMap<RecipeId, f64> {
        self.recipes_vars
            .iter()
            .map(|(iid, var)| {
                let qty = self.solution.value(*var);
                (*iid, qty)
            })
            .filter(|(_, qty)| qty.abs() >= 1e-5)
            .collect()
    }
}

impl<S: LPSolver> Debug for Solution<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "---Solution---")?;
        writeln!(f, "--- Inputs ---")?;
        for (iid, qty) in self.get_inputs() {
            writeln!(f, "item {:#?} : {}", iid, qty)?;
        }
        writeln!(f, "---Outputs---")?;
        for (iid, qty) in self.get_outputs() {
            writeln!(f, "item {:#?} : {}", iid, qty)?;
        }
        writeln!(f, "---Recipes---")?;
        for (rid, qty) in self.get_recipes() {
            writeln!(f, "recipe {:#?} : {}", rid, qty)?;
        }
        Ok(())
    }
}
