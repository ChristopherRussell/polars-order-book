use crate::book_side::{DeleteLevelType, FoundLevelType};
use crate::price_level::{self, QuantityLike};
use thiserror::Error;

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

pub trait PricePointMutationOps<Px: price_level::Price, Qty: QuantityLike> {
    fn add_qty(&mut self, price: Px, qty: Qty) -> FoundLevelType<Qty>;
    fn modify_qty(
        &mut self,
        price: Px,
        qty: Qty,
        prev_price: Px,
        prev_qty: Qty,
    ) -> Result<FoundLevelType<Qty>, PricePointMutationOpsError> {
        self.delete_qty(prev_price, prev_qty)?;
        Ok(self.add_qty(price, qty))
    }
    fn delete_qty(
        &mut self,
        price: Px,
        qty: Qty,
    ) -> Result<DeleteLevelType<Qty>, PricePointMutationOpsError>;
}

pub trait PricePointSummaryOps<Px, Qty> {
    fn set_level(&mut self, price: Px, qty: Qty);
}
