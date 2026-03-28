use std::collections::BTreeMap;
use std::sync::Arc;

use crate::DATA;
use crate::item::{AmountState, InputTab, Items, OutputsTab};
use crate::recipes::RecipeTab;
use crate::{graph_renderer::component::GraphVisualizer, recipes::Recipes};
use leptos::either::EitherOf4;
use leptos::prelude::*;
use solver::recipe::ItemId;
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
    view! {
        <div class="header">
            <TabList selected_value=selected_tab>
                <Tab value=PRODUCTION>
                    "Production"
                </Tab>
                <Tab value=INPUTS>
                    "Inputs"
                </Tab>
                <Tab value=OUTPUTS>
                    "Outputs"
                </Tab>
                <Tab value=RECIPES>
                    "Recipes"
                </Tab>
            </TabList>
            <div class="selectors">
                <div class="selector">
                    <Button on_click=move |_| theme.set(Theme::light())>"Light"</Button>
                </div>
                <div class="selector">
                    <Button on_click=move |_| theme.set(Theme::dark())>"Dark"</Button>
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
            let item = items.items.get(iid)?;
            let amount_state = match item.ressource {
                Some(qty) => AmountState::Some(qty),
                None => AmountState::None,
            };
            Some((*iid, RwSignal::new(amount_state)))
        })
        .collect::<BTreeMap<_, _>>();

    let mut targets = items
        .items
        .keys()
        .filter_map(|iid| {
            let item = items.items.get(iid)?;
            if item.ressource.is_some() {
                return None;
            }
            Some((*iid, RwSignal::new(AmountState::None)))
        })
        .collect::<BTreeMap<_, _>>();

    /* setup */

    let plastic_item_id = ItemId(59);
    let turbo_motor_iid = ItemId(38);

    // leptos::logging::log!("{:#?}", items);

    targets.insert(turbo_motor_iid, RwSignal::new(AmountState::Maximize(623.0)));

    // let crude_oil_id = ItemId(149);
    // let water_id = ItemId(139);

    // available_items.insert(crude_oil_id, RwSignal::new(AmountState::Some(300.0)));
    // available_items.insert(water_id, RwSignal::new(AmountState::Some(f64::MAX)));

    /* end setup */

    let available_items = Arc::new(available_items);
    let targets = Arc::new(targets);
    let selected_recipes = recipes
        .recipes
        .keys()
        .map(|rid| (*rid, RwSignal::new(true)))
        .collect::<BTreeMap<_, _>>();
    let selected_recipes = Arc::new(selected_recipes);
    move || {
        let tab = selected_tab.get();
        let selected_recipes = selected_recipes.clone();
        let available_items = available_items.clone();
        let targets = targets.clone();
        match tab.as_str() {
            PRODUCTION => EitherOf4::A(view! {
                <ContentInner>
                    <GraphVisualizer selected_recipes=selected_recipes available_items=available_items targets=targets />
                </ContentInner>
            }),
            RECIPES => EitherOf4::B(view! {
                <ContentInner>
                    <RecipeTab selected_recipes=selected_recipes />
                </ContentInner>
            }),
            INPUTS => EitherOf4::C(view! {
                <ContentInner>
                    <InputTab available_items=available_items />
                </ContentInner>
            }),
            OUTPUTS => EitherOf4::D(view! {
                <ContentInner>
                    <OutputsTab targets=targets />
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
