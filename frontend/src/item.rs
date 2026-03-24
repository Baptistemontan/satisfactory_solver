use std::{collections::BTreeMap, sync::Arc};

use solver::recipe::ItemId;

#[derive(Debug)]
pub struct Item {
    pub id: ItemId,
    pub icon: Arc<str>,
    pub name: Arc<str>,
    pub description: Arc<str>,
    pub sink_points: f64,
    pub liquid: bool,
}

#[derive(Debug, Clone)]
pub struct Items {
    pub items: Arc<BTreeMap<ItemId, Arc<Item>>>,
}
