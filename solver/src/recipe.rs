use std::collections::BTreeMap;

use good_lp::{Expression, Variable};

use crate::quantity::Quantity;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RecipeId(pub usize);

#[derive(Debug, Clone)]
pub struct Recipe {
    pub inputs: BTreeMap<ItemId, Quantity>,
    pub outputs: BTreeMap<ItemId, Quantity>,
}

impl Recipe {
    pub fn add_to_solver(
        &self,
        variable: Variable,
        items_exprs: &mut BTreeMap<ItemId, Expression>,
    ) {
        for (iid, qty) in &self.inputs {
            let expr = items_exprs
                .entry(*iid)
                .or_insert_with(|| Expression::from(0.0));
            *expr -= *qty * variable;
        }
        for (iid, qty) in &self.outputs {
            let expr = items_exprs
                .entry(*iid)
                .or_insert_with(|| Expression::from(0.0));
            *expr += *qty * variable;
        }
    }
}
