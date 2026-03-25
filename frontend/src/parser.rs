use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::Arc,
};

use serde::{
    Deserialize,
    de::{DeserializeOwned, DeserializeSeed, IgnoredAny, Visitor},
};
use solver::{
    quantity::Quantity,
    recipe::{BuildingId, ItemId, Recipe as SolverRecipe, RecipeId},
};

use crate::{
    buildings::{Building, Buildings},
    item::{Item, Items},
    recipes::{Recipe, Recipes},
};

pub fn parse<R>(reader: R) -> serde_json::error::Result<(Recipes, Items, Buildings)>
where
    R: std::io::Read,
{
    let mut de = serde_json::Deserializer::from_reader(reader);
    let mut main_seed = MainSeed::default();
    DeserializeSeed::deserialize(&mut main_seed, &mut de)?;
    let mut building_recipes: BTreeMap<BuildingId, Arc<BTreeSet<RecipeId>>> = BTreeMap::new();
    for recipe in main_seed.recipes.values() {
        let recipes = building_recipes.entry(recipe.inner.building).or_default();
        let recipes = Arc::get_mut(recipes).unwrap();
        recipes.insert(recipe.id);
    }

    let recipes = Arc::new(main_seed.recipes);
    let items = Arc::new(main_seed.items);
    let buildings = Arc::new(main_seed.buildings);
    let building_recipes = Arc::new(building_recipes);

    Ok((
        Recipes { recipes },
        Items { items },
        Buildings {
            buildings,
            recipes: building_recipes,
        },
    ))
}

#[derive(Default)]
struct MainSeed {
    recipes: BTreeMap<RecipeId, Arc<Recipe>>,
    items: BTreeMap<ItemId, Arc<Item>>,
    item_queue: BTreeMap<Rc<str>, ItemId>,
    building_queue: BTreeMap<Rc<str>, BuildingId>,
    buildings: BTreeMap<BuildingId, Arc<Building>>,
    recipe_id: usize,
    item_id: usize,
    building_id: usize,
}

impl<'de> DeserializeSeed<'de> for &'_ mut MainSeed {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(field_identifier, rename_all = "camelCase")]
enum MainFields {
    Items,
    Recipes,
    Buildings,
    Schematics,
    Generators,
    Resources,
    Miners,
}

impl<'de> Visitor<'de> for &'_ mut MainSeed {
    type Value = ();

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        while let Some(main_field) = map.next_key::<MainFields>()? {
            match main_field {
                MainFields::Items => map.next_value_seed(ItemsSeed { main_seed: self })?,
                MainFields::Recipes => map.next_value_seed(RecipesSeed { main_seed: self })?,
                MainFields::Buildings => map.next_value_seed(BuildingsSeed { main_seed: self })?,
                _ => {
                    // TODO
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        Ok(())
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
    }
}

struct ItemsSeed<'a> {
    main_seed: &'a mut MainSeed,
}

impl<'de> DeserializeSeed<'de> for ItemsSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for ItemsSeed<'_> {
    type Value = ();

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        while let Some(item_classname) = map.next_key::<Rc<str>>()? {
            let item_id = match self.main_seed.item_queue.get(&*item_classname) {
                Some(iid) => *iid,
                None => {
                    let item_id = ItemId(self.main_seed.item_id);
                    self.main_seed.item_id += 1;
                    self.main_seed.item_queue.insert(item_classname, item_id);
                    item_id
                }
            };

            let mut item = map.next_value::<Item>()?;
            item.id = item_id;

            self.main_seed.items.insert(item_id, Arc::new(item));
        }
        Ok(())
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
    }
}

impl<'de> Deserialize<'de> for Item {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ItemVisitor)
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(field_identifier, rename_all = "camelCase")]
enum ItemField {
    Slug,
    Icon,
    Name,
    Description,
    SinkPoints,
    ClassName,
    StackSize,
    EnergyValue,
    RadioactiveDecay,
    Liquid,
    FluidColor,
}

struct ItemVisitor;

fn deserialize_and_set<'de, T: DeserializeOwned, A: serde::de::MapAccess<'de>>(
    value: &mut Option<T>,
    map: &mut A,
) -> Result<(), A::Error> {
    let v = map.next_value()?;
    *value = Some(v);
    Ok(())
}

impl<'de> Visitor<'de> for ItemVisitor {
    type Value = Item;

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut description = None;
        let mut sink_points = None;
        let mut liquid = None;
        let mut name = None;
        let mut icon = None;
        while let Some(field) = map.next_key::<ItemField>()? {
            match field {
                ItemField::Description => deserialize_and_set(&mut description, &mut map)?,
                ItemField::SinkPoints => deserialize_and_set(&mut sink_points, &mut map)?,
                ItemField::Liquid => deserialize_and_set(&mut liquid, &mut map)?,
                ItemField::Name => deserialize_and_set(&mut name, &mut map)?,
                ItemField::Icon => deserialize_and_set(&mut icon, &mut map)?,
                _ => {
                    // TODO
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        // TODO: don't unwrap here
        Ok(Item {
            id: ItemId(usize::MAX),
            icon: icon.unwrap(),
            name: name.unwrap(),
            description: description.unwrap(),
            sink_points: sink_points.unwrap(),
            liquid: liquid.unwrap(),
        })
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a satisfactory item data")
    }
}

struct RecipesSeed<'a> {
    main_seed: &'a mut MainSeed,
}

impl<'de> DeserializeSeed<'de> for RecipesSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for RecipesSeed<'_> {
    type Value = ();

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        while map.next_key::<IgnoredAny>()?.is_some() {
            let recipe = map.next_value_seed(RecipeSeed {
                main_seed: self.main_seed,
            })?;

            let Some(recipe) = recipe else {
                continue;
            };

            self.main_seed.recipes.insert(recipe.id, Arc::new(recipe));
        }
        Ok(())
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(field_identifier, rename_all = "camelCase")]
enum RecipeField {
    Slug,
    Name,
    ClassName,
    Alternate,
    Time,
    InHand,
    ForBuilding,
    InWorkshop,
    InMachine,
    ManualTimeMultiplier,
    Ingredients,
    Products,
    ProducedIn,
    IsVariablePower,
    MinPower,
    MaxPower,
}

struct RecipeSeed<'a> {
    main_seed: &'a mut MainSeed,
}

impl<'de> DeserializeSeed<'de> for RecipeSeed<'_> {
    type Value = Option<Recipe>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for RecipeSeed<'_> {
    type Value = Option<Recipe>;

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut name = None;
        let mut alternate = None;
        let mut inputs = None;
        let mut outputs = None;
        let mut time = None;
        let mut building = None;
        while let Some(field) = map.next_key::<RecipeField>()? {
            match field {
                RecipeField::Name => deserialize_and_set(&mut name, &mut map)?,
                RecipeField::Alternate => deserialize_and_set(&mut alternate, &mut map)?,
                RecipeField::Time => deserialize_and_set(&mut time, &mut map)?,
                RecipeField::Ingredients => {
                    let io = map.next_value_seed(RecipeIoSeed {
                        main_seed: self.main_seed,
                    })?;
                    inputs = Some(io);
                }
                RecipeField::Products => {
                    let io = map.next_value_seed(RecipeIoSeed {
                        main_seed: self.main_seed,
                    })?;
                    outputs = Some(io);
                }
                RecipeField::ProducedIn => {
                    // TODO: skip vec step
                    let mut produced_in = map.next_value::<Vec<Rc<str>>>()?;

                    if let Some(bulding_name) = produced_in.pop() {
                        if let Some(building_id) = self.main_seed.building_queue.get(&*bulding_name)
                        {
                            building = Some(*building_id);
                        } else {
                            let building_id = BuildingId(self.main_seed.building_id);
                            self.main_seed.building_id += 1;
                            self.main_seed
                                .building_queue
                                .insert(bulding_name, building_id);
                            building = Some(building_id);
                        }
                    }
                }
                RecipeField::InMachine => {
                    let in_machine = map.next_value::<bool>()?;
                    if !in_machine {
                        // flush map
                        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                        return Ok(None);
                    }
                }
                _ => {
                    // TODO
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        // TODO: don't unwrap here

        let id = RecipeId(self.main_seed.recipe_id);
        self.main_seed.recipe_id += 1;

        Ok(Some(Recipe {
            id,
            name: name.unwrap(),
            alternate: alternate.unwrap(),
            inner: Arc::new(SolverRecipe {
                inputs: inputs.unwrap(),
                outputs: outputs.unwrap(),
                time: time.unwrap(),
                building: building.unwrap(),
            }),
        }))
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a satisfactory item data")
    }
}

struct RecipeIoSeed<'a> {
    main_seed: &'a mut MainSeed,
}

impl<'de> DeserializeSeed<'de> for RecipeIoSeed<'_> {
    type Value = BTreeMap<ItemId, Quantity>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for RecipeIoSeed<'_> {
    type Value = BTreeMap<ItemId, Quantity>;

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut io = BTreeMap::new();
        while let Some((iid, qty)) = seq.next_element_seed(RecipeIoInner {
            main_seed: self.main_seed,
        })? {
            io.insert(iid, qty);
        }

        Ok(io)
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a list of recipe i/o")
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(field_identifier, rename_all = "camelCase")]
enum RecipeIoField {
    Item,
    Amount,
}

struct RecipeIoInner<'a> {
    main_seed: &'a mut MainSeed,
}

impl<'de> DeserializeSeed<'de> for RecipeIoInner<'_> {
    type Value = (ItemId, Quantity);

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for RecipeIoInner<'_> {
    type Value = (ItemId, Quantity);

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut item: Option<Rc<str>> = None;
        let mut amount = None;
        while let Some(field) = map.next_key::<RecipeIoField>()? {
            match field {
                RecipeIoField::Item => deserialize_and_set(&mut item, &mut map)?,
                RecipeIoField::Amount => deserialize_and_set(&mut amount, &mut map)?,
            }
        }
        // TODO: don't unwrap here
        let item = item.unwrap();
        let item_id = match self.main_seed.item_queue.get(&*item) {
            Some(iid) => *iid,
            None => {
                let iid = ItemId(self.main_seed.item_id);
                self.main_seed.item_id += 1;
                self.main_seed.item_queue.insert(item, iid);
                iid
            }
        };
        let qty = amount.unwrap();
        Ok((item_id, Quantity(qty)))
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a single recipe i/o")
    }
}

struct BuildingsSeed<'a> {
    main_seed: &'a mut MainSeed,
}

impl<'de> DeserializeSeed<'de> for BuildingsSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for BuildingsSeed<'_> {
    type Value = ();

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        while let Some(building_classname) = map.next_key::<Rc<str>>()? {
            let building_id = match self.main_seed.building_queue.get(&*building_classname) {
                Some(bid) => *bid,
                None => {
                    let building_id = BuildingId(self.main_seed.building_id);
                    self.main_seed.building_id += 1;
                    self.main_seed
                        .building_queue
                        .insert(building_classname, building_id);
                    building_id
                }
            };

            let mut building = map.next_value::<Building>()?;
            building.id = building_id;

            self.main_seed
                .buildings
                .insert(building_id, Arc::new(building));
        }
        Ok(())
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
    }
}

impl<'de> Deserialize<'de> for Building {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(BuildingVisitor)
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(field_identifier, rename_all = "camelCase")]
enum BuildingField {
    Slug,
    Icon,
    Name,
    Description,
    ClassName,
    Categories,
    BuildMenuPriority,
    Metadata,
    Size,
}

struct BuildingVisitor;

impl<'de> Visitor<'de> for BuildingVisitor {
    type Value = Building;

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut description = None;
        let mut name = None;
        let mut icon = None;
        while let Some(field) = map.next_key::<BuildingField>()? {
            match field {
                BuildingField::Description => deserialize_and_set(&mut description, &mut map)?,
                BuildingField::Name => deserialize_and_set(&mut name, &mut map)?,
                BuildingField::Icon => deserialize_and_set(&mut icon, &mut map)?,
                _ => {
                    // TODO
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        // TODO: don't unwrap here
        Ok(Building {
            id: BuildingId(usize::MAX),
            icon: icon.unwrap(),
            name: name.unwrap(),
            description: description.unwrap(),
        })
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a satisfactory item data")
    }
}
