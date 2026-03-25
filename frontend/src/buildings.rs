use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use solver::recipe::{BuildingId, RecipeId};

#[derive(Debug)]
pub struct Building {
    pub id: BuildingId,
    pub name: Arc<str>,
    pub icon: Arc<str>,
    pub description: Arc<str>,
}

#[derive(Debug, Clone)]
pub struct Buildings {
    pub buildings: Arc<BTreeMap<BuildingId, Arc<Building>>>,
    pub recipes: Arc<BTreeMap<BuildingId, Arc<BTreeSet<RecipeId>>>>,
}
