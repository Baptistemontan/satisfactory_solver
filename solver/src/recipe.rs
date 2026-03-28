use std::collections::{BTreeMap, BTreeSet};

use good_lp::{Constraint, Expression, ProblemVariables, Variable, constraint, variable};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecipeId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BuildingId(pub usize);

#[derive(Debug, Clone)]
pub struct Recipe {
    pub inputs: BTreeMap<ItemId, f64>,
    pub outputs: BTreeMap<ItemId, f64>,
    pub time: f64,
    pub building: BuildingId,
}

impl Recipe {
    pub fn add_to_fast_solve(
        &self,
        recipe_var: Variable,
        items_expr: &mut BTreeMap<ItemId, Expression>,
    ) {
        let time_coef = 60. / self.time;
        for (iid, qty) in &self.inputs {
            let per_min = time_coef * *qty;
            let expr = items_expr
                .entry(*iid)
                .or_insert_with(|| Expression::from(0.0));
            *expr -= per_min * recipe_var;
        }

        for (iid, qty) in &self.outputs {
            let per_min = time_coef * *qty;
            let expr = items_expr
                .entry(*iid)
                .or_insert_with(|| Expression::from(0.0));
            *expr += per_min * recipe_var;
        }
    }

    fn add_inputs_to_solver(
        &self,
        vars: &mut ProblemVariables,
        rid: RecipeId,
        recip_var: Variable,
        output_expr: &mut BTreeMap<(RecipeId, ItemId), Expression>,
        inputs_expr: &mut BTreeMap<ItemId, Expression>,
        reverse_search: &BTreeMap<ItemId, BTreeSet<RecipeId>>,
        edge_vars: &mut BTreeMap<(RecipeId, RecipeId, ItemId), Variable>,
        in_edges_vars: &mut BTreeMap<(ItemId, RecipeId), Variable>,
        constraints: &mut Vec<Constraint>,
    ) {
        for (iid, qty) in &self.inputs {
            let mut item_expr = Expression::from(0.0);
            let per_min = (60. / self.time) * *qty;
            if let Some(input_expr) = inputs_expr.get_mut(iid) {
                let input_edge_var = vars.add(variable().min(0));
                *input_expr -= input_edge_var;
                in_edges_vars.insert((*iid, rid), input_edge_var);
                item_expr += input_edge_var;
            }
            if let Some(producers) = reverse_search.get(iid) {
                for p_rid in producers {
                    let expr = output_expr
                        .entry((*p_rid, *iid))
                        .or_insert_with(|| Expression::from(0.0));
                    let edge_var = vars.add(variable().min(0));
                    *expr -= edge_var;
                    item_expr += edge_var;
                    edge_vars.insert((*p_rid, rid, *iid), edge_var);
                }
            }
            constraints.push(constraint!(item_expr == per_min * recip_var));
        }
    }

    fn add_outputs_to_solver(
        &self,
        vars: &mut ProblemVariables,
        rid: RecipeId,
        recip_var: Variable,
        output_expr: &mut BTreeMap<(RecipeId, ItemId), Expression>,
        out_edges_vars: &mut BTreeMap<(RecipeId, ItemId), Variable>,
        constraints: &mut Vec<Constraint>,
    ) {
        for (iid, qty) in &self.outputs {
            let per_min = (60. / self.time) * *qty;
            let expr = output_expr
                .entry((rid, *iid))
                .or_insert_with(|| Expression::from(0.0));
            let output_edge_var = vars.add(variable().min(0));
            *expr += output_edge_var;
            constraints.push(constraint!(output_edge_var == per_min * recip_var));
            out_edges_vars.insert((rid, *iid), output_edge_var);
        }
    }

    pub fn add_to_solver(
        &self,
        vars: &mut ProblemVariables,
        rid: RecipeId,
        recip_var: Variable,
        output_expr: &mut BTreeMap<(RecipeId, ItemId), Expression>,
        inputs_expr: &mut BTreeMap<ItemId, Expression>,
        reverse_search: &BTreeMap<ItemId, BTreeSet<RecipeId>>,
        edge_vars: &mut BTreeMap<(RecipeId, RecipeId, ItemId), Variable>,
        in_edges_vars: &mut BTreeMap<(ItemId, RecipeId), Variable>,
        out_edges_vars: &mut BTreeMap<(RecipeId, ItemId), Variable>,
        constraints: &mut Vec<Constraint>,
    ) {
        self.add_inputs_to_solver(
            vars,
            rid,
            recip_var,
            output_expr,
            inputs_expr,
            reverse_search,
            edge_vars,
            in_edges_vars,
            constraints,
        );
        self.add_outputs_to_solver(
            vars,
            rid,
            recip_var,
            output_expr,
            out_edges_vars,
            constraints,
        );
    }
}
