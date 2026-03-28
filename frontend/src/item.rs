use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use solver::recipe::ItemId;

use crate::BASE_URL;
use leptos::{either::Either, prelude::*};
use thaw::{BackTop, Button, Checkbox, Divider, Input, Scrollbar, SpinButton, Switch, Tooltip};

#[derive(Debug)]
pub struct Item {
    pub id: ItemId,
    pub icon: Arc<str>,
    pub name: Arc<str>,
    pub ressource: Option<f64>,
    pub description: Arc<str>,
    pub sink_points: f64,
    pub liquid: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum AmountState {
    None,
    Some(f64),
    Disabled(f64),
    EnabledZero,
    DisabledZero,
    Maximize(f64),
    MaximizeDisabled(f64),
}

#[derive(Debug, Clone)]
pub struct Items {
    pub items: Arc<BTreeMap<ItemId, Arc<Item>>>,
}

#[derive(Debug, Clone, Copy)]
pub struct RessourcesResetProvider(ReadSignal<Option<bool>>);

#[component]
pub fn InputTab(available_items_signal: RwSignal<Vec<(ItemId, AmountState)>>) -> impl IntoView {
    let items = expect_context::<Items>();
    let available_items = available_items_signal.read_untracked();
    let available_items = available_items
        .iter()
        .map(|(iid, qty)| (*iid, *qty))
        .collect::<BTreeMap<_, _>>();
    let amount_signals = items
        .items
        .keys()
        .map(|iid| {
            let amount = available_items
                .get(iid)
                .copied()
                .unwrap_or(AmountState::None);
            (*iid, RwSignal::new(amount))
        })
        .collect::<BTreeMap<_, _>>();

    let mut selected_items = BTreeMap::new();
    {
        for (iid, item) in items.items.iter() {
            if item.ressource.is_some() {
                continue;
            }
            let is_selected = available_items
                .get(iid)
                .is_some_and(|qty| !matches!(qty, AmountState::None));
            // selected_items.insert(*iid, RwSignal::new(is_selected));
            selected_items.insert(*iid, RwSignal::new(is_selected));
        }
    }
    let amount_signals = Arc::new(amount_signals);
    let selected_items = Arc::new(selected_items);

    let ressources = Memo::new({
        let items = items.clone();
        let amount_signals = amount_signals.clone();
        move |_| {
            let mut ressources = Vec::new();
            for (iid, item) in items.items.iter() {
                if item.ressource.is_some() {
                    let amount = amount_signals.get(iid).unwrap();
                    ressources.push((*iid, *amount));
                }
            }
            ressources
        }
    });

    let item_selection = Memo::new({
        let selected_items = selected_items.clone();
        move |old| {
            let mut items: Vec<(ItemId, RwSignal<AmountState>)> = old.cloned().unwrap_or_default();
            let already_selected = items.iter().map(|a| a.0).collect::<BTreeSet<_>>();
            for (iid, selected) in &*selected_items {
                let is_already_selected = already_selected.contains(iid);
                let amount = amount_signals.get(iid).unwrap();
                match (selected.get(), is_already_selected) {
                    (true, true) | (false, false) => {
                        // do nothing
                    }
                    (true, false) => {
                        items.push((*iid, *amount));
                    }
                    (false, true) => {
                        let pos = items.iter().position(|a| a.0 == *iid).unwrap_or(0);
                        items.remove(pos);
                    }
                }
            }
            items
        }
    });

    Effect::new(move |_| {
        let selected_items = item_selection.read();
        let ressources = ressources.read();
        let new_targets = selected_items
            .iter()
            .chain(ressources.iter())
            .map(|(iid, v)| (*iid, v.get()))
            .collect();
        available_items_signal.set(new_targets);
    });

    let reset_signal = RwSignal::new(None);

    let set_ressources_to_zero = move |_| {
        reset_signal.set(Some(false));
    };

    let set_ressources_to_max = move |_| {
        reset_signal.set(Some(true));
    };

    let input_selection = RwSignal::new(false);

    provide_context(RessourcesResetProvider(reset_signal.read_only()));

    view! {
        <div class="input-items-selection">
            {
                let selected_items = selected_items.clone();
                move || {
                    if input_selection.get() {
                        Either::Left(view! {
                            <ItemList selected_items={selected_items.clone()}/>
                        })
                    } else {
                        Either::Right(view! {
                            <div class="input-ressources-selection">
                                <div class="input-ressources-selection-header">
                                    <span>"Ressources"</span>
                                    <div class="input-ressources-selection-toggles">
                                        <Button on_click=set_ressources_to_zero>"Set to 0"</Button>
                                        <Button on_click=set_ressources_to_max>"Set to max"</Button>
                                    </div>
                                </div>
                                <Divider />
                                <ItemsAmountInput item_selection=ressources />
                            </div>
                        })
                    }
                }
            }
            <div class="input-selection-divider">
                <Divider vertical=true />
            </div>
            <div class="input-custom-selection">
                <div class="input-ressources-selection-header">
                    <span>"Inputs"</span>
                    <div class="input-ressources-selection-toggles">
                        // <ItemSelector selected_items={selected_items.clone()} />
                        <Switch checked=input_selection label="Edit Inputs"  />
                    </div>
                </div>
                <Divider />
                <ItemsAmountInput item_selection=item_selection selected=selected_items />
            </div>
        </div>
    }
}

#[component]
pub fn OutputsTab(targets_signal: RwSignal<Vec<(ItemId, AmountState)>>) -> impl IntoView {
    let items = expect_context::<Items>();
    let targets = targets_signal.read_untracked();
    let targets = targets
        .iter()
        .map(|(iid, qty)| (*iid, *qty))
        .collect::<BTreeMap<_, _>>();
    let amount_signals = items
        .items
        .keys()
        .map(|iid| {
            let amount = targets.get(iid).copied().unwrap_or(AmountState::None);
            (*iid, RwSignal::new(amount))
        })
        .collect::<BTreeMap<_, _>>();
    let mut selected_items = BTreeMap::new();
    {
        for (iid, item) in items.items.iter() {
            if item.ressource.is_some() {
                continue;
            }
            let is_selected = targets
                .get(iid)
                .is_some_and(|qty| !matches!(qty, AmountState::None));
            // selected_items.insert(*iid, RwSignal::new(is_selected));
            selected_items.insert(*iid, RwSignal::new(is_selected));
        }
    }
    let amount_signals = Arc::new(amount_signals);
    let selected_items = Arc::new(selected_items);

    let item_selection = Memo::new({
        let selected_items = selected_items.clone();
        move |old| {
            let mut items: Vec<(ItemId, RwSignal<AmountState>)> = old.cloned().unwrap_or_default();
            let already_selected = items.iter().map(|a| a.0).collect::<BTreeSet<_>>();
            for (iid, selected) in &*selected_items {
                let is_already_selected = already_selected.contains(iid);
                let amount = amount_signals.get(iid).unwrap();
                match (selected.get(), is_already_selected) {
                    (true, true) | (false, false) => {
                        // do nothing
                    }
                    (true, false) => {
                        items.push((*iid, *amount));
                    }
                    (false, true) => {
                        let pos = items.iter().position(|a| a.0 == *iid).unwrap_or(0);
                        items.remove(pos);
                    }
                }
            }
            items
        }
    });

    Effect::new(move |_| {
        let selected_items = item_selection.read();
        let new_targets = selected_items
            .iter()
            .map(|(iid, v)| (*iid, v.get()))
            .collect();
        targets_signal.set(new_targets);
    });

    view! {
        <div class="input-items-selection">
            <ItemList selected_items={selected_items.clone()}/>
            <div class="input-selection-divider">
                <Divider vertical=true />
            </div>
            <div class="input-custom-selection">
                <div class="input-ressources-selection-header">
                    <span>"Outputs"</span>
                    <div class="input-ressources-selection-toggles">
                        // <ItemSelector selected_items={selected_items.clone()} />
                        // <Switch checked=input_selection label="Edit Inputs"  />
                    </div>
                </div>
                <Divider />
                <ItemsAmountInput item_selection=item_selection selected=selected_items maximize=true />
            </div>
        </div>
    }
}

// #[component]
// pub fn ItemSelector(selected_items: Arc<BTreeMap<ItemId, RwSignal<bool>>>) -> impl IntoView {
//     let drawer_open = RwSignal::new(false);
//     let on_open = move |_| drawer_open.set(true);
//     let on_close = move |_| drawer_open.set(false);
//     view! {
//         <Button on_click=on_open>"Edit Inputs"</Button>
//         <OverlayDrawer open=drawer_open size=DrawerSize::Medium>
//             <DrawerHeader>
//             <DrawerHeaderTitle>
//                 <DrawerHeaderTitleAction slot>
//                     <Button
//                         appearance=ButtonAppearance::Subtle
//                         on_click=on_close
//                     >
//                         "x"
//                     </Button>
//                 </DrawerHeaderTitleAction>
//                 "Toggle Item"
//             </DrawerHeaderTitle>
//             </DrawerHeader>
//             <DrawerBody>
//                 <ItemList selected_items=selected_items/>
//             </DrawerBody>
//         </OverlayDrawer>
//     }
// }

#[component]
fn ItemList(selected_items: Arc<BTreeMap<ItemId, RwSignal<bool>>>) -> impl IntoView {
    let items = expect_context::<Items>();
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
        <div class="item-picking-list">
            <div class="item-list-header">
                <Input value=search_value placeholder="search" />
            </div>
            <Divider />
            <Scrollbar>
                <div class="item-list-toggles">
                    <For
                        each = move || items_to_display.get()
                        key = |a| a.0
                        let((iid, toggle))
                    >
                        <DisplayItem
                            item_id = iid
                            selected = toggle
                        />
                    </For>
                </div>
                <BackTop/>
            </Scrollbar>
        </div>
    }
}

fn format_icon_href(icon: &str) -> String {
    format!("{}assets/items/{}_64.png", BASE_URL, icon)
}

#[component]
fn DisplayItem(item_id: ItemId, selected: RwSignal<bool>) -> impl IntoView {
    let items = expect_context::<Items>();
    let item = items.items.get(&item_id).unwrap();
    let item_name = item.name.clone();
    let icon_href = format_icon_href(&item.icon);
    view! {
        <div class="item-list-toggle">
            <span>{item_name}</span>
            <img src=icon_href class="item-list-toggle-icon" />
            <Checkbox checked=selected />
        </div>
    }
}

#[component]
pub fn ItemsAmountInput(
    item_selection: Memo<Vec<(ItemId, RwSignal<AmountState>)>>,
    #[prop(optional)] selected: Option<Arc<BTreeMap<ItemId, RwSignal<bool>>>>,
    #[prop(default = false)] maximize: bool,
) -> impl IntoView {
    view! {
        <Scrollbar>
            <div class="items-amount-inputs">
                <For
                    each = move || item_selection.get()
                    key = |a| a.0
                    let((iid, amount))
                >
                    <ItemAmountInput
                        item_id = iid
                        amount = amount
                        selected={selected.clone()}
                        maximize=maximize
                    />
                </For>
            </div>
            <BackTop />
        </Scrollbar>
    }
}

#[component]
fn ItemAmountInput(
    item_id: ItemId,
    amount: RwSignal<AmountState>,
    selected: Option<Arc<BTreeMap<ItemId, RwSignal<bool>>>>,
    maximize: bool,
) -> impl IntoView {
    let reset_ressources_sig = use_context::<RessourcesResetProvider>();
    let items = expect_context::<Items>();
    let item = items.items.get(&item_id).unwrap();
    let name = item.name.clone();
    let icon_href = format_icon_href(&item.icon);
    let ressource_amout = item.ressource;
    let (current_value, activated, currently_maximized) = match amount.get_untracked() {
        AmountState::None | AmountState::EnabledZero => (0.0, true, false),
        AmountState::Some(qty) => (qty, true, false),
        AmountState::Disabled(qty) => (qty, false, false),
        AmountState::DisabledZero => (0.0, false, false),
        AmountState::Maximize(qty) => (qty, true, true),
        AmountState::MaximizeDisabled(qty) => (qty, false, true),
    };
    let current_value = RwSignal::new(current_value);
    let activated = RwSignal::new(activated);

    let delete_button = selected
        .as_ref()
        .and_then(|s| s.get(&item_id))
        .copied()
        .map(|delete| {
            let on_delete = move |_| {
                delete.set(false);
            };
            view! {
                <Button icon=icondata::AiCloseOutlined on_click=on_delete />
            }
        });

    let maximize_status = RwSignal::new(currently_maximized && maximize);

    let maximize_button = bool::then(maximize, move || {
        view! {
            <Tooltip content="Maximize" >
                <Checkbox checked=maximize_status/>
            </Tooltip>
        }
    });

    Effect::new(move || {
        if let Some((RessourcesResetProvider(sig), amount)) =
            reset_ressources_sig.zip(ressource_amout)
        {
            let new_value = match sig.get() {
                Some(true) => amount,
                Some(false) => 0.0,
                None => return,
            };
            current_value.set(new_value);
        }
    });

    Effect::new(move || {
        let qty = current_value.get();
        let active = activated.get();
        let maximize = maximize_status.get();
        let non_zero = qty >= 1e-5;
        match (active, non_zero, maximize) {
            (true, true, false) => {
                amount.set(AmountState::Some(qty));
            }
            (false, true, false) => {
                amount.set(AmountState::Disabled(qty));
            }
            (true, false, false) => {
                amount.set(AmountState::EnabledZero);
            }
            (false, false, false) => {
                amount.set(AmountState::DisabledZero);
            }
            (true, _, true) => {
                amount.set(AmountState::Maximize(qty));
            }
            (false, _, true) => {
                amount.set(AmountState::MaximizeDisabled(qty));
            }
        }
    });

    let amount_input_disabled = move || !activated.get() || maximize_status.get();

    view! {
        <div class="item-amount-input">
            <div class="item-amount-input-item">
                <Checkbox checked=activated/>
                <div>
                    <img src=icon_href class="item-amount-input-icon" />
                </div>
                <span>{name}</span>
            </div>
            <div class="item-amount-input-amount">
                {delete_button}
                <SpinButton<f64> value=current_value step_page=1.0 disabled=amount_input_disabled min=0.0 />
                {maximize_button}
            </div>
        </div>
    }
}
