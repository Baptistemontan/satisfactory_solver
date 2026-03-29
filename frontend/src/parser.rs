use core::f64;
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::Arc,
};

use serde::{
    Deserialize,
    de::{DeserializeOwned, DeserializeSeed, IgnoredAny, Visitor},
};
use solver::recipe::{
    Building as SolverBuilding, BuildingId, ItemId, PowerData, Recipe as SolverRecipe, RecipeId,
};

use crate::{
    buildings::{Building, Buildings},
    item::{Item, Items},
    recipes::{Recipe, Recipes},
};

trait Id: Copy + Ord {
    fn from_usize(id: usize) -> Self;
}

macro_rules! impl_id {
    ($id: ty) => {
        impl Id for $id {
            fn from_usize(id: usize) -> Self {
                Self(id)
            }
        }
    };
}

impl_id!(ItemId);
// impl_id!(RecipeId);
impl_id!(BuildingId);

struct Generator {
    building: BuildingId,
    fuels: BTreeSet<ItemId>,
    power_production: f64,
    water_usage: Option<f64>,
}

fn get_generator_outputs(
    fuel: &Item,
    item_slug: &BTreeMap<Arc<str>, ItemId>,
) -> BTreeMap<ItemId, f64> {
    let (waste_slug, waste_qty) = match &*fuel.slug {
        "uranium-fuel-rod" => ("uranium-waste", 50.0),
        "plutonium-fuel-rod" => ("plutonium-waste", 10.0),
        _ => return BTreeMap::new(),
    };

    let waste_iid = item_slug.get(waste_slug).copied().unwrap();
    BTreeMap::from([(waste_iid, waste_qty)])
}

pub fn parse<R>(reader: R) -> serde_json::error::Result<(Recipes, Items, Buildings)>
where
    R: std::io::Read,
{
    let mut de = serde_json::Deserializer::from_reader(reader);
    let mut main_seed = MainSeed::default();
    DeserializeSeed::deserialize(&mut main_seed, &mut de)?;
    let mut building_recipes: BTreeMap<BuildingId, Arc<BTreeSet<RecipeId>>> = BTreeMap::new();

    let item_slug_search = main_seed
        .items
        .iter()
        .map(|(iid, item)| (item.slug.clone(), *iid))
        .collect::<BTreeMap<_, _>>();

    let water_iid = item_slug_search.get("water").copied().unwrap();

    for generator in &main_seed.generators {
        for fuel in &generator.fuels {
            let recipe_id = RecipeId(main_seed.recipe_id);
            main_seed.recipe_id += 1;

            let fuel_item = main_seed.items.get(fuel).unwrap();
            let time = fuel_item.energy_value / generator.power_production;
            let building = main_seed.buildings.get(&generator.building).unwrap();

            let recipe_name = format!("{} in {}", fuel_item.name, building.name);

            let outputs = get_generator_outputs(fuel_item, &item_slug_search);

            let mut inputs = BTreeMap::from([(*fuel, 1.0)]);
            if let Some(water_usage) = generator.water_usage {
                let amount = time * water_usage;
                inputs.insert(water_iid, amount);
            }

            let recipe = Recipe {
                id: recipe_id,
                name: Arc::from(recipe_name.as_str()),
                alternate: false,
                inner: Arc::new(SolverRecipe {
                    inputs,
                    outputs,
                    time,
                    building: generator.building,
                }),
            };

            main_seed.recipes.insert(recipe_id, Arc::new(recipe));
        }
    }

    for recipe in main_seed.recipes.values() {
        let recipes = building_recipes.entry(recipe.inner.building).or_default();
        let recipes = Arc::get_mut(recipes).unwrap();
        recipes.insert(recipe.id);
    }

    let recipes = Arc::new(main_seed.recipes);
    let items = Arc::new(main_seed.items);
    let buildings = Arc::new(main_seed.buildings);
    let building_recipes = Arc::new(building_recipes);
    let item_slug_search = Arc::new(item_slug_search);

    Ok((
        Recipes { recipes },
        Items {
            items,
            slug_search: item_slug_search,
        },
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
    ressources_queue: BTreeMap<ItemId, Option<f64>>,
    recipe_id: usize,
    item_id: usize,
    building_id: usize,
    generators: Vec<Generator>,
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
                MainFields::Resources => map.next_value_seed(RessourcesSeed { main_seed: self })?,
                MainFields::Generators => {
                    map.next_value_seed(GeneratorsSeed { main_seed: self })?
                }
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
        while let Some(item_id) = map.next_key_seed(IdSeed::<_> {
            queue: &mut self.main_seed.item_queue,
            counter: &mut self.main_seed.item_id,
        })? {
            let seed = ItemSeed {
                main_seed: self.main_seed,
                iid: item_id,
            };

            let item = map.next_value_seed(seed)?;

            self.main_seed.items.insert(item_id, Arc::new(item));
        }
        Ok(())
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
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

struct ItemSeed<'a> {
    iid: ItemId,
    main_seed: &'a mut MainSeed,
}

fn deserialize_and_set<'de, T: DeserializeOwned, A: serde::de::MapAccess<'de>>(
    value: &mut Option<T>,
    map: &mut A,
) -> Result<(), A::Error> {
    let v = map.next_value()?;
    *value = Some(v);
    Ok(())
}

impl<'de> DeserializeSeed<'de> for ItemSeed<'_> {
    type Value = Item;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for ItemSeed<'_> {
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
        let mut slug = None;
        let mut energy_value = None;
        while let Some(field) = map.next_key::<ItemField>()? {
            match field {
                ItemField::Description => deserialize_and_set(&mut description, &mut map)?,
                ItemField::SinkPoints => deserialize_and_set(&mut sink_points, &mut map)?,
                ItemField::Liquid => deserialize_and_set(&mut liquid, &mut map)?,
                ItemField::Name => deserialize_and_set(&mut name, &mut map)?,
                ItemField::Icon => deserialize_and_set(&mut icon, &mut map)?,
                ItemField::Slug => deserialize_and_set(&mut slug, &mut map)?,
                ItemField::EnergyValue => deserialize_and_set(&mut energy_value, &mut map)?,
                _ => {
                    // TODO
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        let ressource = self
            .main_seed
            .ressources_queue
            .get(&self.iid)
            .copied()
            .flatten();

        // TODO: don't unwrap here
        Ok(Item {
            id: self.iid,
            slug: slug.unwrap(),
            icon: icon.unwrap(),
            name: name.unwrap(),
            ressource,
            description: description.unwrap(),
            sink_points: sink_points.unwrap(),
            liquid: liquid.unwrap(),
            energy_value: energy_value.unwrap(),
        })
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a satisfactory item data")
    }
}

struct IdSeed<'a, I, const WITH_CLASS: bool = false> {
    queue: &'a mut BTreeMap<Rc<str>, I>,
    counter: &'a mut usize,
}

impl<'de, I: Id> DeserializeSeed<'de> for IdSeed<'_, I> {
    type Value = I;

    fn deserialize<D>(mut self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        DeserializeSeed::deserialize(&mut self, deserializer)
    }
}

impl<'de, I: Id> DeserializeSeed<'de> for &'_ mut IdSeed<'_, I> {
    type Value = I;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let class_name: Rc<str> = Deserialize::deserialize(deserializer)?;
        let id = match self.queue.get(&class_name) {
            Some(id) => *id,
            None => {
                let id = Id::from_usize(*self.counter);
                *self.counter += 1;
                self.queue.insert(class_name, id);
                id
            }
        };

        Ok(id)
    }
}

impl<'de, I: Id> DeserializeSeed<'de> for IdSeed<'_, I, true> {
    type Value = (I, Rc<str>);

    fn deserialize<D>(mut self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        DeserializeSeed::deserialize(&mut self, deserializer)
    }
}

impl<'de, I: Id> DeserializeSeed<'de> for &'_ mut IdSeed<'_, I, true> {
    type Value = (I, Rc<str>);

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let class_name: Rc<str> = Deserialize::deserialize(deserializer)?;
        let id = match self.queue.get(&class_name) {
            Some(id) => *id,
            None => {
                let id = Id::from_usize(*self.counter);
                *self.counter += 1;
                self.queue.insert(class_name.clone(), id);
                id
            }
        };

        Ok((id, class_name))
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
                    let produced_in = map.next_value_seed(IdListSeed {
                        id_seed: IdSeed {
                            queue: &mut self.main_seed.building_queue,
                            counter: &mut self.main_seed.building_id,
                        },
                    })?;

                    building = produced_in.iter().copied().next();
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
    type Value = BTreeMap<ItemId, f64>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for RecipeIoSeed<'_> {
    type Value = BTreeMap<ItemId, f64>;

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
    type Value = (ItemId, f64);

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for RecipeIoInner<'_> {
    type Value = (ItemId, f64);

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
        Ok((item_id, qty))
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
        while let Some(building_id) = map.next_key_seed(IdSeed::<_> {
            queue: &mut self.main_seed.building_queue,
            counter: &mut self.main_seed.building_id,
        })? {
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
        let mut slug = None;
        while let Some(field) = map.next_key::<BuildingField>()? {
            match field {
                BuildingField::Description => deserialize_and_set(&mut description, &mut map)?,
                BuildingField::Name => deserialize_and_set(&mut name, &mut map)?,
                BuildingField::Icon => deserialize_and_set(&mut icon, &mut map)?,
                BuildingField::Slug => deserialize_and_set(&mut slug, &mut map)?,
                _ => {
                    // TODO
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        // TODO: don't unwrap here
        Ok(Building {
            id: BuildingId(usize::MAX),
            slug: slug.unwrap(),
            icon: icon.unwrap(),
            name: name.unwrap(),
            inner: Arc::new(SolverBuilding {
                power: PowerData::Usage {},
            }),
            description: description.unwrap(),
        })
    }

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a satisfactory item data")
    }
}

struct RessourcesSeed<'a> {
    main_seed: &'a mut MainSeed,
}

impl<'de> DeserializeSeed<'de> for RessourcesSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for RessourcesSeed<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        while let Some(item_class) = map.next_key::<Rc<str>>()? {
            let item_id = match self.main_seed.item_queue.get(&*item_class) {
                Some(iid) => *iid,
                None => {
                    let iid = ItemId(self.main_seed.item_id);
                    self.main_seed.item_id += 1;
                    self.main_seed.item_queue.insert(item_class.clone(), iid);
                    iid
                }
            };
            let amount = get_ress_amount_for(&item_class);
            self.main_seed.ressources_queue.insert(item_id, amount);
            if let Some(item) = self.main_seed.items.get_mut(&item_id) {
                let item = Arc::get_mut(item).unwrap();
                item.ressource = amount;
            }
            map.next_value::<IgnoredAny>()?;
        }

        Ok(())
    }
}

fn get_ress_amount_for(ress: &str) -> Option<f64> {
    match ress {
        "Desc_OreIron_C" => Some(92100.0),
        "Desc_Coal_C" => Some(42300.0),
        "Desc_Water_C" => Some(f64::MAX),
        "Desc_NitrogenGas_C" => Some(12000.0),
        "Desc_Sulfur_C" => Some(10800.0),
        "Desc_SAM_C" => Some(10200.0),
        "Desc_OreBauxite_C" => Some(12300.0),
        "Desc_OreGold_C" => Some(15000.0),
        "Desc_OreCopper_C" => Some(36900.0),
        "Desc_RawQuartz_C" => Some(13500.0),
        "Desc_Stone_C" => Some(69900.0),
        "Desc_OreUranium_C" => Some(2100.0),
        "Desc_LiquidOil_C" => Some(12600.0),
        _ => None,
    }
}

struct GeneratorsSeed<'a> {
    main_seed: &'a mut MainSeed,
}

impl<'de> DeserializeSeed<'de> for GeneratorsSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for GeneratorsSeed<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        while let Some((building_id, building_class)) = map.next_key_seed(IdSeed::<_, true> {
            queue: &mut self.main_seed.building_queue,
            counter: &mut self.main_seed.building_id,
        })? {
            let water_usage = get_generator_water_usage(&building_class);
            let generator = map.next_value_seed(GeneratorSeed {
                main_seed: self.main_seed,
                bid: building_id,
                water_usage,
            })?;
            self.main_seed.generators.push(generator);
        }

        Ok(())
    }
}

fn get_generator_water_usage(generator: &str) -> Option<f64> {
    match generator {
        "Desc_GeneratorCoal_C" => Some(45.0),
        "Desc_GeneratorNuclear_C" => Some(240.0),
        _ => None,
    }
}

struct GeneratorSeed<'a> {
    main_seed: &'a mut MainSeed,
    bid: BuildingId,
    water_usage: Option<f64>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(field_identifier, rename_all = "camelCase")]
enum GeneratorField {
    ClassName,
    Fuel,
    PowerProduction,
    PowerProductionExponent,
    WaterToPowerRatio,
}

impl<'de> DeserializeSeed<'de> for GeneratorSeed<'_> {
    type Value = Generator;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for GeneratorSeed<'_> {
    type Value = Generator;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut fuels = None;
        let mut power_production = None;
        while let Some(gen_field) = map.next_key::<GeneratorField>()? {
            match gen_field {
                GeneratorField::Fuel => {
                    let fuels_items = map.next_value_seed(IdListSeed {
                        id_seed: IdSeed {
                            queue: &mut self.main_seed.item_queue,
                            counter: &mut self.main_seed.item_id,
                        },
                    })?;
                    fuels = Some(fuels_items);
                }
                GeneratorField::PowerProduction => {
                    deserialize_and_set(&mut power_production, &mut map)?
                }
                _ => {
                    // TODO
                    map.next_value::<IgnoredAny>()?;
                }
            }
        }

        Ok(Generator {
            fuels: fuels.unwrap(),
            power_production: power_production.unwrap(),
            building: self.bid,
            water_usage: self.water_usage,
        })
    }
}

struct IdListSeed<'a, I> {
    id_seed: IdSeed<'a, I>,
}

impl<'de, I: Id> DeserializeSeed<'de> for IdListSeed<'_, I> {
    type Value = BTreeSet<I>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, I: Id> Visitor<'de> for IdListSeed<'_, I> {
    type Value = BTreeSet<I>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a sequence of items")
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut ids = BTreeSet::new();
        while let Some(id) = seq.next_element_seed(&mut self.id_seed)? {
            ids.insert(id);
        }

        Ok(ids)
    }
}
