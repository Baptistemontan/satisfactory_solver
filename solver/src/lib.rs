pub mod error;
pub mod quantity;
pub mod recipe;
pub mod solver;

#[cfg(feature = "microlp")]
pub const SOLVER: fn(
    good_lp::variable::UnsolvedProblem,
) -> good_lp::solvers::microlp::MicroLpProblem = good_lp::microlp;

#[cfg(all(not(feature = "microlp"), feature = "clarabel"))]
pub const SOLVER: fn(
    good_lp::variable::UnsolvedProblem,
) -> good_lp::solvers::clarabel::ClarabelProblem = good_lp::clarabel;

#[cfg(all(test, any(feature = "microlp", feature = "clarabel")))]
mod tests {
    use super::SOLVER;
    use crate::{
        quantity::Quantity,
        recipe::{BuildingId, ItemId, Recipe, RecipeId},
        solver::{Solver, Target},
    };
    use core::f64;
    use float_eq::assert_float_eq;
    use std::{collections::BTreeMap, rc::Rc, sync::Arc};

    #[test]
    fn test_optimize() {
        let iron_ore = ItemId(0);
        let iron_ingot = ItemId(1);
        let iron_plate = ItemId(2);
        let iron_ingot_recipe_id = RecipeId(0);
        let iron_plate_recipe_id = RecipeId(1);
        let iron_ingot_recipe = Arc::new(Recipe {
            inputs: BTreeMap::from([(iron_ore, Quantity(1.))]),
            outputs: BTreeMap::from([(iron_ingot, Quantity(1.))]),
            time: 2.,
            building: BuildingId(0),
        });
        let iron_plate_recipe = Arc::new(Recipe {
            inputs: BTreeMap::from([(iron_ingot, Quantity(3.))]),
            outputs: BTreeMap::from([(iron_plate, Quantity(2.))]),
            time: 6.,
            building: BuildingId(0),
        });

        let availables = BTreeMap::from([(iron_ore, Quantity(f64::INFINITY))]);
        let recipes = BTreeMap::from([
            (iron_ingot_recipe_id, iron_ingot_recipe),
            (iron_plate_recipe_id, iron_plate_recipe),
        ]);
        let target_qty = 35.;
        let target = Target {
            iid: iron_plate,
            qty: Some(target_qty.into()),
        };

        let solution = Solver::new(&recipes)
            .optimize(SOLVER, &[target], &availables)
            .unwrap();

        let inputs = solution.get_inputs();
        assert_eq!(inputs.len(), 1);
        let used_qty = inputs.get(&iron_ore).unwrap();
        assert_float_eq!(*used_qty, target_qty * 3. / 2., abs <= 1e-5);

        let outputs = solution.get_outputs();
        assert_eq!(outputs.len(), 1);
        let produced_qty = outputs.get(&iron_plate).unwrap();
        assert_float_eq!(*produced_qty, target_qty, abs <= 1e-5);

        let recipes = solution.get_recipes();
        assert_eq!(recipes.len(), 2);
        let iron_ingot_recipe_count = recipes.get(&iron_ingot_recipe_id).unwrap();
        assert_float_eq!(*iron_ingot_recipe_count, target_qty / 20., abs <= 1e-5);
        let iron_plate_recipe_count = recipes.get(&iron_plate_recipe_id).unwrap();
        assert_float_eq!(*iron_plate_recipe_count, target_qty / 20., abs <= 1e-5);
    }

    #[test]
    fn test_maximize() {
        let available_ores = 120.;

        let iron_ore = ItemId(0);
        let iron_ingot = ItemId(1);
        let iron_plate = ItemId(2);
        let iron_ingot_recipe_id = RecipeId(0);
        let iron_plate_recipe_id = RecipeId(1);
        let iron_ingot_recipe = Arc::new(Recipe {
            inputs: BTreeMap::from([(iron_ore, Quantity(1.))]),
            outputs: BTreeMap::from([(iron_ingot, Quantity(1.))]),
            time: 2.,
            building: BuildingId(0),
        });
        let iron_plate_recipe = Arc::new(Recipe {
            inputs: BTreeMap::from([(iron_ingot, Quantity(3.))]),
            outputs: BTreeMap::from([(iron_plate, Quantity(2.))]),
            time: 6.,
            building: BuildingId(0),
        });

        let availables = BTreeMap::from([(iron_ore, Quantity(available_ores))]);
        let recipes = BTreeMap::from([
            (iron_ingot_recipe_id, iron_ingot_recipe),
            (iron_plate_recipe_id, iron_plate_recipe),
        ]);
        let target = Target {
            iid: iron_plate,
            qty: None,
        };

        let solution = Solver::new(&recipes)
            .optimize(SOLVER, &[target], &availables)
            .unwrap();

        let inputs = solution.get_inputs();
        assert_eq!(inputs.len(), 1);
        let used_qty = inputs.get(&iron_ore).unwrap();
        assert_float_eq!(*used_qty, available_ores, abs <= 1e-5);

        let outputs = solution.get_outputs();
        assert_eq!(outputs.len(), 1);
        let produced_qty = outputs.get(&iron_plate).unwrap();
        assert_float_eq!(*produced_qty, available_ores * 2. / 3., abs <= 1e-5);

        let recipes = solution.get_recipes();
        assert_eq!(recipes.len(), 2);
        let iron_ingot_recipe_count = recipes.get(&iron_ingot_recipe_id).unwrap();
        assert_float_eq!(*iron_ingot_recipe_count, available_ores / 30., abs <= 1e-5);
        let iron_plate_recipe_count = recipes.get(&iron_plate_recipe_id).unwrap();
        assert_float_eq!(*iron_plate_recipe_count, available_ores / 30., abs <= 1e-5);
    }

    #[test]
    fn test_feedback() {
        let available_item = 100.;

        let item1 = ItemId(0);
        let item2 = ItemId(1);
        let recipe_id = RecipeId(0);
        let recipe = Arc::new(Recipe {
            inputs: BTreeMap::from([(item1, Quantity(2.))]),
            outputs: BTreeMap::from([(item1, Quantity(1.)), (item2, Quantity(1.))]),
            time: 60.,
            building: BuildingId(0),
        });
        let availables = BTreeMap::from([(item1, Quantity(available_item))]);
        let recipes = BTreeMap::from([(recipe_id, recipe)]);
        let target = Target {
            iid: item2,
            qty: None,
        };

        let solution = Solver::new(&recipes)
            .optimize(SOLVER, &[target], &availables)
            .unwrap();

        let inputs = solution.get_inputs();
        assert_eq!(inputs.len(), 1);
        let used_qty = inputs.get(&item1).unwrap();
        assert_float_eq!(*used_qty, available_item, abs <= 1e-5);

        let outputs = solution.get_outputs();
        assert_eq!(outputs.len(), 1);
        let produced_qty = outputs.get(&item2).unwrap();
        assert_float_eq!(*produced_qty, available_item, abs <= 1e-5);

        let recipes = solution.get_recipes();
        assert_eq!(recipes.len(), 1);
        let recipe_qty = recipes.get(&recipe_id).unwrap();
        assert_float_eq!(*recipe_qty, available_item, abs <= 1e-5);
    }

    #[test]
    fn test_infinite_feedback() {
        let item1 = ItemId(0);
        let item2 = ItemId(1);
        let recipe_id = RecipeId(0);
        let recipe = Arc::new(Recipe {
            inputs: BTreeMap::from([(item1, Quantity(1.))]),
            outputs: BTreeMap::from([(item1, Quantity(1.)), (item2, Quantity(1.))]),
            time: 60.,
            building: BuildingId(0),
        });
        let availables = BTreeMap::new();
        let recipes = BTreeMap::from([(recipe_id, recipe)]);

        let qty = f64::MAX;
        let target = Target {
            iid: item2,
            qty: Some(qty.into()),
        };

        let solution = Solver::new(&recipes)
            .optimize(SOLVER, &[target], &availables)
            .unwrap();

        let inputs = solution.get_inputs();
        assert_eq!(inputs.len(), 0);

        let outputs = solution.get_outputs();
        assert_eq!(outputs.len(), 1);
        let produced_qty = outputs.get(&item2).unwrap();
        assert_float_eq!(*produced_qty, f64::MAX, abs <= 1e-5);

        let recipes = solution.get_recipes();
        assert_eq!(recipes.len(), 1);
        let recipe_qty = recipes.get(&recipe_id).unwrap();
        assert_float_eq!(*recipe_qty, f64::MAX, abs <= 1e-5);
    }
}
