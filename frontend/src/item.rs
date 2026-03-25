use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use solver::recipe::ItemId;

use crate::{BASE_URL, utils::ArcIter};
use leptos::prelude::*;
use thaw::{
    Button, ButtonAppearance, Checkbox, DrawerBody, DrawerHeader, DrawerHeaderTitle,
    DrawerHeaderTitleAction, Input, OverlayDrawer,
};

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

#[component]
pub fn ItemSelector(
    items: Items,
    selected_items: Arc<BTreeMap<ItemId, RwSignal<bool>>>,
) -> impl IntoView {
    let drawer_open = RwSignal::new(false);
    let on_open = move |_| drawer_open.set(true);
    let on_close = move |_| drawer_open.set(false);
    view! {
        <Button on_click=on_open>"Add Item"</Button>
        <OverlayDrawer open=drawer_open>
            <DrawerHeader>
            <DrawerHeaderTitle>
                <DrawerHeaderTitleAction slot>
                    <Button
                        appearance=ButtonAppearance::Subtle
                        on_click=on_close
                    >
                        "x"
                    </Button>
                </DrawerHeaderTitleAction>
                "Select Item"
            </DrawerHeaderTitle>
            </DrawerHeader>
            <DrawerBody>
                <ItemList items=items selected_items=selected_items/>
            </DrawerBody>
        </OverlayDrawer>
    }
}

#[component]
fn ItemList(items: Items, selected_items: Arc<BTreeMap<ItemId, RwSignal<bool>>>) -> impl IntoView {
    let search_value = RwSignal::new(String::new());

    let search_helper = selected_items
        .iter()
        .filter_map(|(iid, _)| items.items.get(iid))
        .map(|item| (item.name.to_lowercase(), item.id))
        .collect::<HashMap<String, ItemId>>();

    let items_to_display = RwSignal::<BTreeMap<_, _>>::new(Default::default());

    Effect::new(move |_| {
        let searched = search_value.read().to_lowercase();
        if searched.is_empty() {
            items_to_display.set(BTreeMap::clone(&selected_items));
            return;
        }

        let mut to_display = BTreeSet::new();

        for (name, iid) in &search_helper {
            if name.contains(&*searched) {
                to_display.insert(*iid);
            }
        }
        let to_display = to_display
            .iter()
            .filter_map(|iid| Some((*iid, *selected_items.get(iid)?)))
            .collect();
        items_to_display.set(to_display);
    });

    view! {
        <div>
            <Input value=search_value placeholder="search" />
            <For
                each = move || items_to_display.get()
                key = |a| a.0
                let((iid, toggle))
            >
                <DisplayItem
                    item_id = iid
                    items = {items.clone()}
                    selected = toggle
                />
            </For>
        </div>
    }
}

fn format_icon_href(icon: &str) -> String {
    format!("{}assets/items/{}_64.png", BASE_URL, icon)
}

#[component]
fn DisplayItem(item_id: ItemId, items: Items, selected: RwSignal<bool>) -> impl IntoView {
    let item = items.items.get(&item_id).unwrap();
    let item_name = item.name.clone();
    let icon_href = format_icon_href(&item.icon);
    view! {
        <div class="item-selection">
            <span>{item_name}</span>
            <img src=icon_href class="item-selection-icon" />
            <Checkbox checked=selected />
        </div>
    }
}
