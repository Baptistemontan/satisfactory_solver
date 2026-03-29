use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use solver::recipe::{Building as BuildingInner, BuildingId, RecipeId};

#[derive(Debug)]
pub struct Building {
    pub id: BuildingId,
    pub slug: Arc<str>,
    pub name: Arc<str>,
    pub icon: Arc<str>,
    pub description: Arc<str>,
    pub inner: Arc<BuildingInner>,
}

#[derive(Debug, Clone)]
pub struct Buildings {
    pub buildings: Arc<BTreeMap<BuildingId, Arc<Building>>>,
    pub recipes: Arc<BTreeMap<BuildingId, Arc<BTreeSet<RecipeId>>>>,
}
