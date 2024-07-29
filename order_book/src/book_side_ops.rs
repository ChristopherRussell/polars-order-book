use thiserror::Error;

use crate::book_side::{DeleteLevelType, FoundLevelType};
use crate::price_level::PriceLevel;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum LevelError {
    #[error("Level not found")]
    LevelNotFound,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum PricePointMutationOpsError {
    #[error(transparent)]
    LevelError(#[from] LevelError),
    #[error("Qty exceeds available")]
    QtyExceedsAvailable,
}

pub trait PricePointMutationOps<Price, Qty> {
    fn add_qty(&mut self, price: Price, qty: Qty) -> (FoundLevelType, PriceLevel<Price, Qty>);
    fn modify_qty(
        &mut self,
        price: Price,
        qty: Qty,
        prev_price: Price,
        prev_qty: Qty,
    ) -> Result<(FoundLevelType, PriceLevel<Price, Qty>), PricePointMutationOpsError> {
        self.delete_qty(prev_price, prev_qty)?;
        Ok(self.add_qty(price, qty))
    }
    fn delete_qty(
        &mut self,
        price: Price,
        qty: Qty,
    ) -> Result<(DeleteLevelType, PriceLevel<Price, Qty>), PricePointMutationOpsError>;
}

pub trait PricePointSummaryOps<Price, Qty> {
    fn set_level(&mut self, price: Price, qty: Qty);
}
