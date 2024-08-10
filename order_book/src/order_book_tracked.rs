use crate::book_side_tracked::BookSideWithTopNTracking;
use order_book_core::price_level::{AskPrice, BidPrice, PriceLike, QuantityLike};
use order_book_derive::BidAskBook;
use std::fmt::Debug;

#[derive(BidAskBook)]
pub struct OrderBookWithTopNTracking<Px: PriceLike, Qty: QuantityLike, const N: usize> {
    pub asks: BookSideWithTopNTracking<AskPrice<Px>, Qty, N>,
    pub bids: BookSideWithTopNTracking<BidPrice<Px>, Qty, N>,
}

impl<Px: PriceLike, Qty: QuantityLike, const N: usize> Debug
    for OrderBookWithTopNTracking<Px, Qty, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OrderBookWithTopNTracking-{}Tracking {{ Bids: {:?}, Asks: {:?} }}",
            N, self.bids, self.asks
        )
    }
}

impl<Px: PriceLike, Qty: QuantityLike, const N: usize> Default
    for OrderBookWithTopNTracking<Px, Qty, N>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Px: PriceLike, Qty: QuantityLike, const N: usize> OrderBookWithTopNTracking<Px, Qty, N> {
    pub fn new() -> Self {
        OrderBookWithTopNTracking {
            bids: BookSideWithTopNTracking::new(),
            asks: BookSideWithTopNTracking::new(),
        }
    }
}
