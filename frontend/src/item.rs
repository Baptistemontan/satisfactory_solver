use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use solver::recipe::ItemId;
use web_sys::MouseEvent;

use crate::i18n::*;
use crate::{BASE_URL, t_s};
use leptos::{either::Either, prelude::*};
use thaw::{
    BackTop, Button, Checkbox, Divider, Icon, Input, Scrollbar, SpinButton, Switch, Tooltip,
};

#[derive(Debug)]
pub struct Item {
    pub id: ItemId,
    pub slug: Arc<str>,
    pub icon: Arc<str>,
    pub name: Arc<str>,
    pub ressource: Option<f64>,
    pub description: Arc<str>,
    pub sink_points: f64,
    pub liquid: bool,
}

// #[derive(Debug, Clone, Copy, Default)]
// pub enum AmountState {
//     #[default]
//     Unselected,
//     Enabled(f64),
//     Disabled(f64),
//     EnabledDefault,
//     DisabledDefault,
//     Maximize(f64),
//     MaximizeDisabled(f64),
// }

#[derive(Debug, Clone)]
pub struct Items {
    pub items: Arc<BTreeMap<ItemId, Arc<Item>>>,
    pub slug_search: Arc<BTreeMap<Arc<str>, ItemId>>,
}

#[derive(Debug, Clone, Copy)]
pub struct RessourcesResetProvider(ReadSignal<Option<bool>>);

fn iter_signals<'a, K, T>(
    iter: impl IntoIterator<Item = &'a K>,
    signals: &BTreeMap<K, RwSignal<T>>,
) -> impl Iterator<Item = (K, RwSignal<T>)>
where
    K: Ord + Copy + 'a,
{
    IntoIterator::into_iter(iter).filter_map(|iid| {
        let sig = signals.get(iid)?;
        Some((*iid, *sig))
    })
}

#[component]
pub fn InputTab(
    available_items_signal: RwSignal<Vec<ItemId>>,
    amount_signals: Arc<BTreeMap<ItemId, RwSignal<f64>>>,
    cost_signals: Arc<BTreeMap<ItemId, RwSignal<f64>>>,
    input_enabled: Arc<BTreeMap<ItemId, RwSignal<bool>>>,
) -> impl IntoView {
    let i18n = use_i18n();

    let items = expect_context::<Items>();

    let ressources = items
        .items
        .iter()
        .filter(|(_, item)| item.ressource.is_some())
        .map(|(iid, _)| *iid)
        .collect::<Vec<_>>();

    let ressources = RwSignal::new(ressources);

    let mut selected_items = items
        .items
        .keys()
        .map(|iid| (*iid, RwSignal::new(false)))
        .collect::<BTreeMap<_, _>>();

    available_items_signal.with_untracked(|items| {
        for iid in items {
            selected_items.insert(*iid, RwSignal::new(true));
        }
    });

    let selected_items = Arc::new(selected_items);

    Effect::new({
        let selected_items = selected_items.clone();
        let input_enabled = input_enabled.clone();
        move |_| {
            let mut to_add = BTreeSet::new();
            for (iid, sig) in &*selected_items {
                if sig.get() {
                    to_add.insert(*iid);
                }
            }
            let mut ai = available_items_signal.write();

            let mut to_remove = BTreeSet::new();

            for iid in &*ai {
                if !to_add.contains(iid) {
                    to_remove.insert(*iid);
                }
                to_add.remove(iid);
            }

            for iid in &to_add {
                let enabled_sig = input_enabled.get(iid).unwrap();
                enabled_sig.set(true);
            }

            for iid in &to_remove {
                let enabled_sig = input_enabled.get(iid).unwrap();
                enabled_sig.set(false);
            }
            ai.retain(|iid| !to_remove.contains(iid));
            ai.extend(to_add);
        }
    });

    let item_cost_input = RwSignal::new(false);

    // let draggable_list = RwSignal::new(None);
    // Effect::new(move |_| {
    //     let Some((from, to)) = draggable_list.get() else {
    //         return;
    //     };
    //     leptos::logging::log!("from {} to {}", from, to);
    //     draggable_list.update_untracked(|v| *v = None);

    //     for sig in [available_items_signal, item_cost_selection] {
    //         let mut writer = sig.write();
    //         leptos::logging::log!("from {:?}", &*writer);
    //         let value = writer.remove(from);
    //         writer.insert(to, value);
    //         leptos::logging::log!("to {:?}", &*writer);
    //     }
    // });

    let reset_signal = RwSignal::new(None);

    let ressources_default_amounts = items
        .items
        .iter()
        .filter_map(|(iid, item)| item.ressource.map(|qty| (*iid, qty)))
        .collect::<BTreeMap<_, _>>();
    let ressources_default_amounts = Arc::new(ressources_default_amounts);

    let set_ressources_to_zero = {
        let ressources_default_amounts = ressources_default_amounts.clone();
        let amount_signals = amount_signals.clone();
        move |_| {
            for iid in ressources_default_amounts.keys() {
                let qty = amount_signals.get(iid).unwrap();
                qty.set(0.0);
            }
        }
    };

    let set_ressources_to_max = {
        let amount_signals = amount_signals.clone();
        move |_| {
            for (iid, amount) in &*ressources_default_amounts {
                let qty = amount_signals.get(iid).unwrap();
                qty.set(*amount);
            }
        }
    };

    let input_selection = RwSignal::new(false);

    let amounts = move || {
        if item_cost_input.get() {
            cost_signals.clone()
        } else {
            amount_signals.clone()
        }
    };

    provide_context(RessourcesResetProvider(reset_signal.read_only()));

    view! {
        <div class="input-items-selection">
            {
                let enabled_inputs = input_enabled.clone();
                let amounts = amounts.clone();
                let selected_items = selected_items.clone();
                move || {
                    let input_selection = input_selection.get();
                    let cost_input = item_cost_input.get();
                    let amounts = amounts.clone()();
                    let enabled_inputs = enabled_inputs.clone();
                    let selected_items = selected_items.clone();
                    let set_ressources_to_zero = set_ressources_to_zero.clone();
                    let set_ressources_to_max = set_ressources_to_max.clone();
                    if input_selection {
                        Either::Left(view! {
                            <ItemList selected_items=selected_items/>
                        })
                    } else {
                        Either::Right(view! {
                            <div class="input-ressources-selection">
                                <div class="input-ressources-selection-header">
                                    <span>{t!(i18n, ressources.ressources)}</span>
                                    <div class="input-ressources-selection-toggles">
                                        <Button
                                            on_click = set_ressources_to_zero
                                            disabled = cost_input
                                        >
                                            {t!(i18n, ressources.set_to_zero)}
                                        </Button>
                                        <Button
                                            on_click = set_ressources_to_max
                                            disabled = cost_input
                                        >
                                            {t!(i18n, ressources.set_to_max)}
                                        </Button>
                                    </div>
                                </div>
                                <Divider />
                                <ItemsAmountInput
                                    items = ressources
                                    amounts = amounts
                                    enabled = enabled_inputs
                                />
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
                    <span>{t!(i18n, inputs.inputs)}</span>
                    <div class="input-ressources-selection-toggles">
                        <Switch checked=item_cost_input label={t_s!(i18n, inputs.edit_costs)}  />
                        <Switch checked=input_selection label={t_s!(i18n, inputs.edit_inputs)}  />
                    </div>
                </div>
                <Divider />
                {
                    move || {
                        let amounts = amounts();
                        let enabled_inputs = input_enabled.clone();
                        let selected_items = selected_items.clone();
                        view! {
                            <ItemsAmountInput
                                items = available_items_signal
                                amounts = amounts
                                enabled = enabled_inputs
                                selected = selected_items
                            />
                        }
                    }
                }
            </div>
        </div>
    }
}

#[component]
pub fn OutputsTab(
    targets_signal: RwSignal<Vec<ItemId>>,
    amount_signals: Arc<BTreeMap<ItemId, RwSignal<f64>>>,
    enabled_signals: Arc<BTreeMap<ItemId, RwSignal<bool>>>,
    maximize_signals: Arc<BTreeMap<ItemId, RwSignal<bool>>>,
) -> impl IntoView {
    let i18n = use_i18n();

    let items = expect_context::<Items>();

    let mut selected_items = items
        .items
        .keys()
        .map(|iid| (*iid, RwSignal::new(false)))
        .collect::<BTreeMap<_, _>>();

    targets_signal.with_untracked(|items| {
        for iid in items {
            selected_items.insert(*iid, RwSignal::new(true));
        }
    });

    let selected_items = Arc::new(selected_items);

    Effect::new({
        let selected_items = selected_items.clone();
        let input_enabled = enabled_signals.clone();
        move |_| {
            let mut to_add = BTreeSet::new();
            for (iid, sig) in &*selected_items {
                if sig.get() {
                    to_add.insert(*iid);
                }
            }
            let mut ai = targets_signal.write();

            let mut to_remove = BTreeSet::new();

            for iid in &*ai {
                if !to_add.contains(iid) {
                    to_remove.insert(*iid);
                }
                to_add.remove(iid);
            }

            for iid in &to_add {
                let enabled_sig = input_enabled.get(iid).unwrap();
                enabled_sig.set(true);
            }

            for iid in &to_remove {
                let enabled_sig = input_enabled.get(iid).unwrap();
                enabled_sig.set(false);
            }
            ai.retain(|iid| !to_remove.contains(iid));
            ai.extend(to_add);
        }
    });

    // let targets = targets_signal.read_untracked();
    // let targets = targets
    //     .iter()
    //     .map(|(iid, qty)| (*iid, *qty))
    //     .collect::<BTreeMap<_, _>>();
    // let amount_signals = items
    //     .items
    //     .keys()
    //     .map(|iid| {
    //         let amount = targets.get(iid).copied().unwrap_or(AmountState::Unselected);
    //         (*iid, RwSignal::new(amount))
    //     })
    //     .collect::<BTreeMap<_, _>>();
    // let mut selected_items = BTreeMap::new();
    // {
    //     for (iid, item) in items.items.iter() {
    //         if item.ressource.is_some() {
    //             continue;
    //         }
    //         let is_selected = targets
    //             .get(iid)
    //             .is_some_and(|qty| !matches!(qty, AmountState::Unselected));
    //         // selected_items.insert(*iid, RwSignal::new(is_selected));
    //         selected_items.insert(*iid, RwSignal::new(is_selected));
    //     }
    // }
    // let amount_signals = Arc::new(amount_signals);
    // let selected_items = Arc::new(selected_items);

    // let item_selection = RwSignal::new(Vec::<(ItemId, RwSignal<AmountState>)>::new());

    // Effect::new({
    //     let selected_items = selected_items.clone();
    //     move |_| {
    //         let mut items_guard = item_selection.write();
    //         let already_selected = items_guard.iter().map(|a| a.0).collect::<BTreeSet<_>>();
    //         for (iid, selected) in &*selected_items {
    //             let is_already_selected = already_selected.contains(iid);
    //             let amount = amount_signals.get(iid).unwrap();
    //             match (selected.get(), is_already_selected) {
    //                 (true, true) | (false, false) => {
    //                     // do nothing
    //                 }
    //                 (true, false) => {
    //                     items_guard.push((*iid, *amount));
    //                 }
    //                 (false, true) => {
    //                     let pos = items_guard.iter().position(|a| a.0 == *iid).unwrap_or(0);
    //                     items_guard.remove(pos);
    //                 }
    //             }
    //         }
    //     }
    // });

    // Effect::new(move |_| {
    //     let selected_items = item_selection.read();
    //     let new_targets = selected_items
    //         .iter()
    //         .map(|(iid, v)| (*iid, v.get()))
    //         .collect();
    //     targets_signal.set(new_targets);
    // });

    view! {
        <div class="input-items-selection">
            <ItemList selected_items={selected_items.clone()}/>
            <div class="input-selection-divider">
                <Divider vertical=true />
            </div>
            <div class="input-custom-selection">
                <div class="input-ressources-selection-header">
                    <span>{t!(i18n, outputs.outputs)}</span>
                    <div class="input-ressources-selection-toggles">
                        // <ItemSelector selected_items={selected_items.clone()} />
                        // <Switch checked=input_selection label="Edit Inputs"  />
                    </div>
                </div>
                <Divider />
                <ItemsAmountInput
                    items = targets_signal
                    amounts = amount_signals
                    enabled = enabled_signals
                    maximize = maximize_signals
                    selected = selected_items
                    movable = true
                />
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
    let i18n = use_i18n();
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
                <Input value=search_value placeholder={t_s!(i18n, search)} />
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
    items: RwSignal<Vec<ItemId>>,
    amounts: Arc<BTreeMap<ItemId, RwSignal<f64>>>,
    enabled: Arc<BTreeMap<ItemId, RwSignal<bool>>>,
    #[prop(optional)] selected: Option<Arc<BTreeMap<ItemId, RwSignal<bool>>>>,
    #[prop(optional)] maximize: Option<Arc<BTreeMap<ItemId, RwSignal<bool>>>>,
    #[prop(default = false)] movable: bool,
) -> impl IntoView {
    let drag_index: RwSignal<Option<usize>> = RwSignal::new(None);
    let hover_index: RwSignal<Option<usize>> = RwSignal::new(None);

    let on_drop = move |_| {
        if let (Some(from), Some(to)) = (drag_index.get(), hover_index.get())
            && movable
            && from != to
        {
            leptos::logging::log!("dropped {} | {}", from, to);
            items.update(|list| {
                let value = list.remove(from);
                list.insert(to, value);
            });
        }
        drag_index.set(None);
        hover_index.set(None);
    };

    let on_mouse_leave = move |_| {
        drag_index.set(None);
        hover_index.set(None);
    };

    let each = move || {
        items.with(|items| {
            items
                .iter()
                .map(|iid| {
                    let amount_sig = amounts.get(iid).copied().unwrap();
                    let enabled = enabled.get(iid).copied().unwrap();
                    let selected_sig = selected
                        .as_ref()
                        .map(|selected| selected.get(iid).unwrap())
                        .copied();
                    let maximize = maximize
                        .as_ref()
                        .map(|maximize| maximize.get(iid).unwrap())
                        .copied();
                    (*iid, amount_sig, enabled, selected_sig, maximize)
                })
                .collect::<Vec<_>>()
        })
    };

    let index = move |iid: ItemId| {
        items
            .with(|items| items.iter().position(|i| *i == iid))
            .unwrap()
    };

    view! {
        <Scrollbar>
            <ul
                class="items-amount-inputs"
                class=("items-amount-inputs-grabbed", move || drag_index.get().is_some())
                on:mouseup=on_drop
                on:mouseleave=on_mouse_leave
            >
                <For
                    each = each
                    key = |a| a.0
                    let((iid, amount, enabled, selected, maximize))
                >
                    <ItemAmountInput
                        item_id = iid
                        amount = amount
                        enabled = enabled
                        selected = selected
                        maximize = maximize
                        movable={movable.then_some((drag_index, hover_index))}
                        idx = move || index(iid)
                    />
                </For>
            </ul>
            <BackTop />
        </Scrollbar>
    }
}

#[component]
fn ItemAmountInput(
    item_id: ItemId,
    amount: RwSignal<f64>,
    selected: Option<RwSignal<bool>>,
    maximize: Option<RwSignal<bool>>,
    enabled: RwSignal<bool>,
    #[prop(into)] idx: Signal<usize>,
    movable: Option<(RwSignal<Option<usize>>, RwSignal<Option<usize>>)>,
) -> impl IntoView {
    // Drag icons:
    // AiMenuOutlined
    // BiExpandVerticalRegular
    // BiMoveVerticalRegular
    // BsArrowDownUp
    // BsArrowsExpand
    // BsChevronBarExpand

    let i18n = use_i18n();

    let items = expect_context::<Items>();
    let item = items.items.get(&item_id).unwrap();
    let name = item.name.clone();
    let icon_href = format_icon_href(&item.icon);

    let delete_button = selected.map(|selected_sig| {
        let on_delete = move |_| {
            selected_sig.set(false);
        };
        view! {
            <Button icon=icondata::AiCloseOutlined on_click=on_delete />
        }
    });

    let maximize_button = maximize.map(|maximize_status| {
        view! {
            <Tooltip content={t_s!(i18n, inputs.maximize)}>
                <Checkbox checked=maximize_status/>
            </Tooltip>
        }
    });

    let amount_input_disabled = move || {
        let enabled = enabled.get();
        let maximize = maximize.is_some_and(|sig| sig.get());
        let selected = selected.map(|sig| sig.get()).unwrap_or(true);
        !selected || !enabled || maximize
    };

    let grab_icon = if let Some((drag_idx, _)) = movable {
        let on_click = move |e: MouseEvent| {
            if e.button() != 0 {
                return;
            }
            let idx = idx.get();
            leptos::logging::log!("grabbed {}", idx);
            drag_idx.set(Some(idx))
        };
        let grabbable = move || drag_idx.get().is_none();
        let view = view! {
            <div on:mousedown=on_click class="item-amount-input-grab-icon" class=("item-amount-input-draggable", grabbable)>
                <Icon icon=icondata::BsChevronBarExpand />
            </div>
        };
        Some(view)
    } else {
        None
    };

    let drag_over = move |e: MouseEvent| {
        if let Some((_, hover_idx)) = movable {
            let idx = idx.get();
            leptos::logging::log!("hovered {}", idx);
            e.prevent_default();
            hover_idx.set(Some(idx))
        }
    };

    // let is_grabbed = move || movable.is_some_and(|(di, _)| di.get() == Some(idx.get()));

    view! {
        <li
            class="item-amount-input"
            // class=("item-amount-input-draggable", move || movable.is_some() && !is_grabbed())
            // class=("item-amount-input-grabbed", move || is_grabbed())
            // draggable={movable.is_some()}
            on:mouseenter=drag_over
        >
            <div class="item-amount-input-item">
                {grab_icon}
                <Checkbox checked=enabled />
                <div>
                    <img src=icon_href class="item-amount-input-icon" />
                </div>
                <span>{name}</span>
            </div>
            <div class="item-amount-input-amount">
                {delete_button}
                <SpinButton<f64> class="item-amount-input-spin-button" value=amount step_page=1.0 disabled=amount_input_disabled min=0.0 />
                {maximize_button}
            </div>
        </li>
    }
}
