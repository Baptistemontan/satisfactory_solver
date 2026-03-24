use std::{collections::BTreeMap, sync::Arc};

use solver::{
    quantity::Quantity,
    recipe::{ItemId, Recipe as SolverRecipe, RecipeId},
};

#[derive(Debug)]
pub struct Recipe {
    pub id: RecipeId,
    pub name: Arc<str>,
    pub alternate: bool,
    pub inner: Arc<SolverRecipe>,
}

impl Recipe {
    pub fn inputs(&self) -> &BTreeMap<ItemId, Quantity> {
        &self.inner.inputs
    }

    pub fn outputs(&self) -> &BTreeMap<ItemId, Quantity> {
        &self.inner.outputs
    }

    pub fn time(&self) -> f64 {
        self.inner.time
    }
}

#[derive(Debug, Clone)]
pub struct Recipes {
    pub recipes: Arc<BTreeMap<RecipeId, Arc<Recipe>>>,
}

impl Recipes {
    pub fn get(&self, rid: RecipeId) -> Option<Arc<Recipe>> {
        self.recipes.get(&rid).cloned()
    }
}
