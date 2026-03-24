use std::{collections::BTreeMap, sync::Arc};

use solver::recipe::BuildingId;

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
}
