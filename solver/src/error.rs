use crate::recipe::ItemId;
use good_lp::{Solver as LPSolver, SolverModel};
use thiserror::Error;

type LPError<S> = <<S as LPSolver>::Model as SolverModel>::Error;
pub type Result<T, S> = core::result::Result<T, Error<S>>;

#[derive(Debug, Error)]
pub enum Error<S: LPSolver> {
    #[error(transparent)]
    SolverError(LPError<S>),
    #[error("No recipe for item with id {0:#?}")]
    NoRecipe(ItemId),
    #[error("No target set for the solver")]
    NoTarget,
}
