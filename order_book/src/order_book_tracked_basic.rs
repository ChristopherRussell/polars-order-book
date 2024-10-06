use crate::book_side_tracked_basic::BookSideWithBasicTracking;
use order_book_core::price_level::{AskPrice, BidPrice, PriceLike, QuantityLike};
use order_book_derive::BidAskBook;

#[derive(BidAskBook)]
pub struct OrderBookWithBasicTracking<Px: PriceLike, Qty: QuantityLike> {
    asks: BookSideWithBasicTracking<AskPrice<Px>, Qty>,
    bids: BookSideWithBasicTracking<BidPrice<Px>, Qty>,
}
impl<Px: PriceLike, Qty: QuantityLike> OrderBookWithBasicTracking<Px, Qty> {
    pub fn new() -> Self {
        OrderBookWithBasicTracking {
            asks: BookSideWithBasicTracking::new(),
            bids: BookSideWithBasicTracking::new(),
        }
    }
}

impl<Px: PriceLike, Qty: QuantityLike> Default for OrderBookWithBasicTracking<Px, Qty> {
    fn default() -> Self {
        Self::new()
    }
}
