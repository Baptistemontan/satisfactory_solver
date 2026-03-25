use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
};

use solver::{
    quantity::Quantity,
    recipe::{ItemId, Recipe as SolverRecipe, RecipeId},
};

use leptos::prelude::*;
use thaw::{BackTop, Button, Checkbox, Divider, Input, Scrollbar, Tooltip};

use crate::{
    BASE_URL,
    item::{Item, Items},
};
#[derive(Debug)]
pub struct Recipe {
    pub id: RecipeId,
    pub name: Arc<str>,
    pub alternate: bool,
    pub inner: Arc<SolverRecipe>,
}

impl Recipe {
    pub fn inputs(&self) -> &BTreeMap<ItemId, Quantity> {
        &self.inner.inputs
    }

    pub fn outputs(&self) -> &BTreeMap<ItemId, Quantity> {
        &self.inner.outputs
    }

    pub fn time(&self) -> f64 {
        self.inner.time
    }
}

#[derive(Debug, Clone)]
pub struct Recipes {
    pub recipes: Arc<BTreeMap<RecipeId, Arc<Recipe>>>,
}

impl Recipes {
    pub fn get(&self, rid: RecipeId) -> Option<Arc<Recipe>> {
        self.recipes.get(&rid).cloned()
    }

    pub fn default_selected_recipes(&self) -> BTreeMap<RecipeId, Arc<SolverRecipe>> {
        self.recipes
            .iter()
            .map(|(rid, r)| (*rid, r.inner.clone()))
            .collect()
    }
}

#[component]
pub fn RecipeTab(selected_recipes: Arc<BTreeMap<RecipeId, RwSignal<bool>>>) -> impl IntoView {
    let recipes = expect_context::<Recipes>();

    let alternate_recipes = recipes
        .recipes
        .iter()
        .filter(|(_, r)| r.alternate)
        .map(|(rid, r)| (*rid, r.clone()))
        .collect::<BTreeMap<RecipeId, Arc<Recipe>>>();

    let base_recipes = recipes
        .recipes
        .iter()
        .filter(|(_, r)| !r.alternate)
        .map(|(rid, r)| (*rid, r.clone()))
        .collect::<BTreeMap<RecipeId, Arc<Recipe>>>();

    view! {
        <Scrollbar>
            <div class="recipes">
                <div class="base-recipes">
                    <RecipeList recipes={Arc::new(base_recipes)} selected_recipes={selected_recipes.clone()} />
                </div>
                <div class="recipes-divider">
                    <Divider vertical=true />
                </div>
                <div class="alternate-recipes">
                    <RecipeList recipes={Arc::new(alternate_recipes)} selected_recipes=selected_recipes />
                </div>
            </div>
            <BackTop />
        </Scrollbar>
    }
}
#[component]
pub fn RecipeList(
    selected_recipes: Arc<BTreeMap<RecipeId, RwSignal<bool>>>,
    recipes: Arc<BTreeMap<RecipeId, Arc<Recipe>>>,
) -> impl IntoView {
    let search_value = RwSignal::new(String::new());
    let recipes_to_display: RwSignal<BTreeMap<_, _>> = RwSignal::new(Default::default());
    let mut search_helper = recipes
        .iter()
        .map(|(rid, r)| (r.name.to_lowercase(), BTreeSet::from([*rid])))
        .collect::<HashMap<String, BTreeSet<RecipeId>>>();
    let items = expect_context::<Items>();
    for (rid, recipe) in &*recipes {
        let io = recipe.inputs().keys().chain(recipe.outputs().keys());
        for iid in io {
            let item = items.items.get(iid).unwrap();
            search_helper
                .entry(item.name.to_lowercase())
                .or_default()
                .insert(*rid);
        }
    }

    let on_toggle_all = {
        let recipes = recipes.clone();
        let selected_recipes = selected_recipes.clone();
        move |_| {
            for rid in recipes.keys() {
                let recipe_toggle = selected_recipes.get(rid).unwrap();
                recipe_toggle.set(true);
            }
        }
    };

    let on_toggle_none = {
        let recipes = recipes.clone();
        let selected_recipes = selected_recipes.clone();
        move |_| {
            for rid in recipes.keys() {
                let recipe_toggle = selected_recipes.get(rid).unwrap();
                recipe_toggle.set(false);
            }
        }
    };

    Effect::new(move |_| {
        let searched = search_value.read().to_lowercase();
        if searched.is_empty() {
            recipes_to_display.set(BTreeMap::clone(&recipes));
            return;
        }

        let mut to_display = BTreeSet::new();

        for (name, rids) in &search_helper {
            if name.contains(&*searched) {
                to_display.extend(rids.iter().copied());
            }
        }
        let to_display = to_display
            .iter()
            .filter_map(|rid| Some((*rid, recipes.get(rid)?.clone())))
            .collect();
        recipes_to_display.set(to_display);
    });

    view! {
        <div class="recipe-list">
            <div class="recipe-list-header">
                <Input value=search_value placeholder="search" />
                <div class="recipe-list-toggles">
                    <div class="recipe-list-toggle">
                        <Button on_click=on_toggle_all>"All"</Button>
                    </div>
                    <div class="recipe-list-toggle">
                        <Button on_click=on_toggle_none>"None"</Button>
                    </div>
                </div>
            </div>
            <For
                each = move || recipes_to_display.get().clone()
                key = |a| a.0
                let((rid, recipe))
            >
                <RecipePicker
                    rid=rid
                    selected_recipes={selected_recipes.clone()}
                    recipe=recipe
                    items={items.clone()}
                />
            </For>
        </div>
    }
}

#[component]
pub fn RecipePicker(
    selected_recipes: Arc<BTreeMap<RecipeId, RwSignal<bool>>>,
    rid: RecipeId,
    recipe: Arc<Recipe>,
    items: Items,
) -> impl IntoView {
    let selected = *selected_recipes.get(&rid).unwrap();

    let inputs = display_io(&items, recipe.inputs());
    let outputs = display_io(&items, recipe.outputs());

    view! {
        <div class="recipe-picker">
            <div class="recipe-checkbox">
                <Checkbox checked=selected />
                <span>{recipe.name.clone()}</span>
            </div>
            <div class="recipe-io">
                <div class="recipe-inputs">
                    {inputs}
                </div>
                <span>" = "</span>
                <div class="recipe-outputs">
                    {outputs}
                </div>
            </div>
        </div>
    }
}

fn display_io(items: &Items, io: &BTreeMap<ItemId, Quantity>) -> Option<impl IntoView + use<>> {
    let mut items = io
        .iter()
        .filter_map(|(iid, qty)| Some((items.items.get(iid)?.clone(), qty.0 as i32)));

    let (first_item, first_qty) = items.next()?;
    let first = display_item(first_item, first_qty);
    let others = items
        .map(|(item, qty)| (recipe_io_sep(), display_item(item, qty)))
        .collect::<Vec<_>>();
    Some((first, others))
}

fn recipe_io_sep() -> impl IntoView {
    // view! {
    //     <span class="recipe-io-sep">"+"</span>
    // }
}

fn format_icon_href(icon: &str) -> String {
    format!("{}assets/items/{}_64.png", BASE_URL, icon)
}

fn display_item(item: Arc<Item>, qty: i32) -> impl IntoView {
    let icon_href = format_icon_href(&item.icon);

    view! {
        <div class="recipe-io-item">
            <span>"x"{qty}</span>
            <Tooltip content={item.name.to_string()}>
                <img src=icon_href class="recipe-io-icon" />
            </Tooltip>
        </div>
    }
}
