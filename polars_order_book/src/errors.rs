use order_book_core::book_side_ops::PricePointMutationOpsError;
use polars::error::{ErrString, PolarsError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum PolarsOrderBookError {
    #[error(transparent)]
    PricePointMutationOpsError(#[from] PricePointMutationOpsError),
}

impl From<PolarsOrderBookError> for PolarsError {
    fn from(e: PolarsOrderBookError) -> Self {
        PolarsError::ComputeError(ErrString::from(e.to_string()))
    }
}