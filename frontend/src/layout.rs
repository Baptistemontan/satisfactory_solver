use std::collections::BTreeMap;
use std::sync::Arc;

use crate::i18n::*;
use crate::item::{AmountState, InputTab, Items, OutputsTab};
use crate::recipes::RecipeTab;
use crate::{graph_renderer::component::GraphVisualizer, recipes::Recipes};
use leptos::either::EitherOf4;
use leptos::prelude::*;
use thaw::{Button, Tab, TabList, Theme};

const PRODUCTION: &str = "production";
const RECIPES: &str = "recipes";
const INPUTS: &str = "inputs";
const OUTPUTS: &str = "outputs";

#[component]
pub fn Layout(theme: RwSignal<Theme>) -> impl IntoView {
    let selected_tab = RwSignal::new(PRODUCTION.to_string());
    view! {
        <div class="layout">
            <Header selected_tab=selected_tab theme=theme />
            <Content selected_tab=selected_tab />
            <Footer />
        </div>
    }
}

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <div class="footer">
            "Footer"
        </div>
    }
}

#[component]
pub fn Header(selected_tab: RwSignal<String>, theme: RwSignal<Theme>) -> impl IntoView {
    let i18n = use_i18n();
    view! {
        <div class="header">
            <TabList selected_value=selected_tab>
                <Tab value=PRODUCTION>
                    {t!(i18n, tabs.production)}
                </Tab>
                <Tab value=INPUTS>
                    {t!(i18n, tabs.inputs)}
                </Tab>
                <Tab value=OUTPUTS>
                    {t!(i18n, tabs.outputs)}
                </Tab>
                <Tab value=RECIPES>
                    {t!(i18n, tabs.recipes)}
                </Tab>
            </TabList>
            <div class="selectors">
                <div class="selector">
                    <Button on_click=move |_| theme.set(Theme::light())>{t!(i18n, theme.light)}</Button>
                </div>
                <div class="selector">
                    <Button on_click=move |_| theme.set(Theme::dark())>{t!(i18n, theme.dark)}</Button>
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn Content(selected_tab: RwSignal<String>) -> impl IntoView {
    let items = expect_context::<Items>();
    let recipes = expect_context::<Recipes>();
    let available_items = items
        .items
        .keys()
        .filter_map(|iid| {
            items
                .items
                .get(iid)
                .and_then(|item| item.ressource)
                .map(|qty| (*iid, AmountState::Some(qty)))
        })
        .collect::<Vec<_>>();

    let mut targets = Vec::new();

    let mut item_costs = items
        .items
        .keys()
        .map(|iid| (*iid, RwSignal::new(AmountState::Some(1.0))))
        .collect::<BTreeMap<_, _>>();

    // set water to no cost by default
    let water_iid = items.slug_search.get("water").copied().unwrap();
    item_costs.insert(water_iid, RwSignal::new(AmountState::Some(0.0)));

    /* debug setup */

    let plastic_item_id = items.slug_search.get("plastic").copied().unwrap();

    // leptos::logging::log!("{:#?}", items);

    targets.push((plastic_item_id, AmountState::Maximize(0.0)));

    // let crude_oil_id = ItemId(149);
    // let water_id = ItemId(139);

    // available_items.insert(crude_oil_id, RwSignal::new(AmountState::Some(300.0)));
    // available_items.insert(water_id, RwSignal::new(AmountState::Some(f64::MAX)));

    /* end setup */

    let available_items = RwSignal::new(available_items);
    let targets = RwSignal::new(targets);
    let selected_recipes = recipes
        .recipes
        .keys()
        .map(|rid| (*rid, RwSignal::new(true)))
        .collect::<BTreeMap<_, _>>();
    let selected_recipes = Arc::new(selected_recipes);
    let item_costs = Arc::new(item_costs);
    move || {
        let tab = selected_tab.get();
        let selected_recipes = selected_recipes.clone();
        let item_costs = item_costs.clone();
        match tab.as_str() {
            PRODUCTION => EitherOf4::A(view! {
                <ContentInner>
                    <GraphVisualizer
                        selected_recipes=selected_recipes
                        available_items=available_items
                        targets=targets
                        item_cost=item_costs
                    />
                </ContentInner>
            }),
            RECIPES => EitherOf4::B(view! {
                <ContentInner>
                    <RecipeTab selected_recipes=selected_recipes />
                </ContentInner>
            }),
            INPUTS => EitherOf4::C(view! {
                <ContentInner>
                    <InputTab available_items_signal=available_items item_costs=item_costs />
                </ContentInner>
            }),
            OUTPUTS => EitherOf4::D(view! {
                <ContentInner>
                    <OutputsTab targets_signal=targets />
                </ContentInner>
            }),
            _ => unreachable!(),
        }
    }
}

#[component]
fn ContentInner<V: IntoView>(children: TypedChildren<V>) -> impl IntoView {
    let c = children.into_inner();

    view! {
        <div class="content">
            {c()}
        </div>
    }
}
