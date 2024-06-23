use thiserror::Error;

use crate::book_side::{DeleteLevelType, FoundLevelType};
use crate::price_level::PriceLevel;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum LevelError {
    #[error("Level not found")]
    LevelNotFound,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum DeleteError {
    #[error(transparent)]
    LevelError(#[from] LevelError),
    #[error("Qty exceeds available")]
    QtyExceedsAvailable,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum BookSideOpsError {
    #[error(transparent)]
    DeleteError(#[from] DeleteError),
    // #[error("Qty exceeds available")]
    // QtyExceedsAvailable,
}
pub trait BookSideOps<Price, Qty> {
    fn add_qty(&mut self, price: Price, qty: Qty) -> (FoundLevelType, PriceLevel<Price, Qty>);
    fn modify_qty(
        &mut self,
        price: Price,
        qty: Qty,
        prev_price: Price,
        prev_qty: Qty,
    ) -> Result<(FoundLevelType, PriceLevel<Price, Qty>), BookSideOpsError> {
        self.delete_qty(prev_price, prev_qty)?;
        Ok(self.add_qty(price, qty))
    }
    fn delete_qty(
        &mut self,
        price: Price,
        qty: Qty,
    ) -> Result<(DeleteLevelType, PriceLevel<Price, Qty>), BookSideOpsError>;
}
