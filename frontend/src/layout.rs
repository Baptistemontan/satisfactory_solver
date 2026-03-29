use std::collections::BTreeMap;
use std::sync::Arc;

use crate::i18n::*;
use crate::item::{InputTab, Items, OutputsTab};
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

    let selected_recipes = recipes
        .recipes
        .keys()
        .map(|rid| (*rid, RwSignal::new(true)))
        .collect::<BTreeMap<_, _>>();

    let amount_signals = items
        .items
        .iter()
        .map(|(iid, item)| {
            let amount = item.ressource.unwrap_or(0.0);
            (*iid, RwSignal::new(amount))
        })
        .collect::<BTreeMap<_, _>>();

    let mut cost_signals = items
        .items
        .keys()
        .map(|iid| (*iid, RwSignal::new(1.0)))
        .collect::<BTreeMap<_, _>>();

    let mut targets_amount_signals = items
        .items
        .keys()
        .map(|iid| (*iid, RwSignal::new(0.0)))
        .collect::<BTreeMap<_, _>>();

    let input_enabled = items
        .items
        .iter()
        .map(|(iid, item)| (*iid, RwSignal::new(item.ressource.is_some())))
        .collect::<BTreeMap<_, _>>();

    let mut output_enabled = items
        .items
        .iter()
        .map(|(iid, item)| (*iid, RwSignal::new(item.ressource.is_some())))
        .collect::<BTreeMap<_, _>>();

    let mut output_maximized = items
        .items
        .keys()
        .map(|iid| (*iid, RwSignal::new(false)))
        .collect::<BTreeMap<_, _>>();

    let available_items = Vec::new();
    let mut targets = Vec::new();

    // set water to no cost by default
    let water_iid = items.slug_search.get("water").copied().unwrap();
    cost_signals.insert(water_iid, RwSignal::new(0.0));

    /* debug setup */

    let plastic_item_id = items.slug_search.get("plastic").copied().unwrap();
    targets_amount_signals.insert(plastic_item_id, RwSignal::new(0.0));
    output_enabled.insert(plastic_item_id, RwSignal::new(true));
    output_maximized.insert(plastic_item_id, RwSignal::new(true));
    targets.push(plastic_item_id);

    // leptos::logging::log!("{:#?}", items);

    // let crude_oil_id = ItemId(149);
    // let water_id = ItemId(139);

    // available_items.insert(crude_oil_id, RwSignal::new(AmountState::Some(300.0)));
    // available_items.insert(water_id, RwSignal::new(AmountState::Some(f64::MAX)));

    /* end setup */

    let available_items = RwSignal::new(available_items);
    let targets = RwSignal::new(targets);

    let selected_recipes = Arc::new(selected_recipes);
    let amount_signals = Arc::new(amount_signals);
    let cost_signals = Arc::new(cost_signals);
    let targets_amount_signals = Arc::new(targets_amount_signals);
    let output_maximized = Arc::new(output_maximized);
    let output_enabled = Arc::new(output_enabled);
    let input_enabled = Arc::new(input_enabled);
    move || {
        let tab = selected_tab.get();
        let selected_recipes = selected_recipes.clone();
        let amount_signals = amount_signals.clone();
        let cost_signals = cost_signals.clone();
        let targets_amount_signals = targets_amount_signals.clone();
        let output_maximized = output_maximized.clone();
        let output_enabled = output_enabled.clone();
        let input_enabled = input_enabled.clone();
        match tab.as_str() {
            PRODUCTION => EitherOf4::A(view! {
                <ContentInner>
                    <GraphVisualizer
                        selected_recipes=selected_recipes
                        targets=targets
                        availables_amount_signals=amount_signals
                        cost_signals=cost_signals
                        target_signals=targets_amount_signals
                        output_maximized=output_maximized
                        output_enabled=output_enabled
                        input_enabled=input_enabled
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
                    <InputTab
                        available_items_signal=available_items
                        amount_signals=amount_signals
                        cost_signals=cost_signals
                        input_enabled=input_enabled
                    />
                </ContentInner>
            }),
            OUTPUTS => EitherOf4::D(view! {
                <ContentInner>
                    <OutputsTab
                        targets_signal=targets
                        amount_signals=targets_amount_signals
                        enabled_signals=output_enabled
                        maximize_signals=output_maximized
                    />
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
