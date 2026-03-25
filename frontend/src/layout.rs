use std::collections::BTreeMap;
use std::sync::Arc;

use crate::item::{ItemSelector, Items};
use crate::recipes::RecipeTab;
use crate::{graph_renderer::component::GraphVisualizer, recipes::Recipes};
use leptos::either::EitherOf3;
use leptos::{either::Either, prelude::*};
use solver::recipe::ItemId;
use thaw::{Button, Tab, TabList, Theme};

const PRODUCTION: &str = "production";
const RECIPES: &str = "recipes";
const INPUTS: &str = "inputs";

#[component]
pub fn Layout(theme: RwSignal<Theme>) -> impl IntoView {
    let selected_tab = RwSignal::new(INPUTS.to_string());
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
    let recipes = expect_context::<Recipes>();
    let items = expect_context::<Items>();
    let selected_items = items
        .items
        .keys()
        .map(|iid| (*iid, RwSignal::new(false)))
        .collect::<BTreeMap<_, _>>();
    let selected_items = Arc::new(selected_items);
    let selected_recipes = recipes
        .recipes
        .keys()
        .map(|rid| (*rid, RwSignal::new(true)))
        .collect::<BTreeMap<_, _>>();
    let selected_recipes = Arc::new(selected_recipes);
    move || {
        let tab = selected_tab.get();
        let selected_recipes = selected_recipes.clone();
        match tab.as_str() {
            PRODUCTION => EitherOf3::A(view! {
                <ContentInner>
                    <GraphVisualizer selected_recipes=selected_recipes />
                </ContentInner>
            }),
            RECIPES => EitherOf3::B(view! {
                <ContentInner>
                    <RecipeTab selected_recipes=selected_recipes />
                </ContentInner>
            }),
            INPUTS => EitherOf3::C(view! {
                <ItemSelector items={items.clone()} selected_items={selected_items.clone()} />
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
