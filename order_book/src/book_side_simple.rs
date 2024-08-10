use hashbrown::HashMap;
use order_book_core::book_side::{BookSide, DeleteLevelType, FoundLevelType};
use order_book_core::book_side_ops::{PricePointMutationOps, PricePointMutationOpsError};
use order_book_core::price_level::{self, AskPrice, BidPrice, PriceLike, QuantityLike};
use order_book_derive::BookSide;
use std::fmt::Debug;
use tracing::{debug, instrument};

use order_book_core;
#[derive(BookSide)]
pub struct SimpleBookSide<Px: price_level::Price, Qty: QuantityLike> {
    pub levels: HashMap<Px, Qty>,
}

// TODO: Created to simplify tests, but didn't work out. Remove usages.
pub enum BidOrAskSimpleBookSide<Px: PriceLike, Qty: QuantityLike> {
    Ask(SimpleBookSide<AskPrice<Px>, Qty>),
    Bid(SimpleBookSide<BidPrice<Px>, Qty>),
}

impl<Px: PriceLike, Qty: QuantityLike> Debug for BidOrAskSimpleBookSide<Px, Qty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BidOrAskSimpleBookSide::Ask(ask) => write!(f, "Ask({:?})", ask),
            BidOrAskSimpleBookSide::Bid(bid) => write!(f, "Bid({:?})", bid),
        }
    }
}

impl<Px: price_level::Price, Qty: QuantityLike> SimpleBookSide<Px, Qty> {
    pub fn new() -> Self {
        SimpleBookSide {
            levels: HashMap::new(),
        }
    }
}
impl<Px: price_level::Price, Qty: QuantityLike> Default for SimpleBookSide<Px, Qty> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Px: price_level::Price, Qty: QuantityLike> Debug for SimpleBookSide<Px, Qty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} BookSide {{ levels: {:?} }}",
            Px::SIDE_NAME,
            self.levels
        )
    }
}

impl<Px: price_level::Price, Qty: QuantityLike> PricePointMutationOps<Px, Qty>
    for SimpleBookSide<Px, Qty>
{
    #[instrument]
    #[inline]
    fn add_qty(&mut self, price: Px, qty: Qty) -> FoundLevelType<Qty> {
        self.find_or_create_level_and_add_qty(price, qty)
    }

    #[instrument]
    #[inline]
    fn delete_qty(
        &mut self,
        price: Px,
        qty: Qty,
    ) -> Result<DeleteLevelType<Qty>, PricePointMutationOpsError> {
        debug!("Called delete_qty");
        self.remove_qty_from_level_and_maybe_delete(price, qty)
    }
}
