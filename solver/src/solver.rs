use crate::{
    error::{Result, try_solver_err},
    quantity::Quantity,
    recipe::{ItemId, Recipe, RecipeId},
};
use good_lp::{
    Expression, ProblemVariables, Solution as GLPSolution, Solver as LPSolver, SolverModel,
    Variable, constraint, variable, variables,
};
use std::{collections::BTreeMap, fmt::Debug};

#[derive(Clone)]
pub struct Solver {
    vars: ProblemVariables,
    recipes_vars: BTreeMap<RecipeId, Variable>,
    items_exprs: BTreeMap<ItemId, Expression>,
}

pub struct Target {
    pub iid: ItemId,
    pub qty: Option<Quantity>,
}

pub struct Solution<S> {
    pub(crate) solution: S,
    pub(crate) recipes_vars: BTreeMap<RecipeId, Variable>,
    pub(crate) input_vars: BTreeMap<ItemId, Variable>,
    pub(crate) sinks: BTreeMap<ItemId, Variable>,
}

type LPSolution<S> = <<S as LPSolver>::Model as SolverModel>::Solution;

pub type SolutionResult<S> = Result<Solution<LPSolution<S>>, S>;

impl Solver {
    pub fn new(recipes: &BTreeMap<RecipeId, &Recipe>) -> Self {
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
        target: Target,
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

        let Some(target_sink) = sinks.get(&target.iid) else {
            todo!("no recipe for item {:?}", target.iid);
        };

        let mut problem = match target.qty {
            Some(qty) => {
                let mut inputs_expr = Expression::from(0.0);
                for input_var in input_vars.values() {
                    inputs_expr += input_var;
                }
                let constraint = constraint!(*target_sink == qty);
                vars.minimise(inputs_expr).using(solver).with(constraint)
            }
            None => vars.maximise(target_sink).using(solver),
        };

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

    pub fn maximise<S: LPSolver + Clone>(
        self,
        solver: S,
        target: ItemId,
        availables: &BTreeMap<ItemId, Quantity>,
    ) -> SolutionResult<S> {
        let maximize_target = Target {
            iid: target,
            qty: None,
        };
        let maximized_solution = self
            .clone()
            .solve(solver.clone(), maximize_target, availables)?;
        let Some(target_var) = maximized_solution.sinks.get(&target) else {
            todo!("target var not found");
        };

        let max_production = maximized_solution.solution.value(*target_var);

        self.optimize(solver, target, Quantity(max_production), availables)
    }

    pub fn optimize<S: LPSolver>(
        self,
        solver: S,
        target: ItemId,
        qty: impl Into<Quantity>,
        availables: &BTreeMap<ItemId, Quantity>,
    ) -> SolutionResult<S> {
        let target = Target {
            iid: target,
            qty: Some(qty.into()),
        };
        self.solve(solver, target, availables)
    }
}

impl<S: GLPSolution> Solution<S> {
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

impl<S: GLPSolution> Debug for Solution<S> {
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
