use crate::book_side_simple::SimpleBookSide;
use order_book_core::price_level::{AskPrice, BidPrice, PriceLike, QuantityLike};
use order_book_derive::BidAskBook;

#[derive(BidAskBook)]
pub struct SimpleOrderBook<Px: PriceLike, Qty: QuantityLike> {
    asks: SimpleBookSide<AskPrice<Px>, Qty>,
    bids: SimpleBookSide<BidPrice<Px>, Qty>,
}

impl<Px: PriceLike, Qty: QuantityLike> SimpleOrderBook<Px, Qty> {
    pub fn new() -> Self {
        SimpleOrderBook {
            bids: SimpleBookSide::new(),
            asks: SimpleBookSide::new(),
        }
    }
}

impl<Px: PriceLike, Qty: QuantityLike> Default for SimpleOrderBook<Px, Qty> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use order_book_core::book_side::BookSide;

    // test default and new

    #[test]
    fn test_default() {
        let book: SimpleOrderBook<i64, i64> = Default::default();
        assert!(book.asks.levels().is_empty());
        assert!(book.bids.levels().is_empty());
    }

    #[test]
    fn test_new() {
        let book: SimpleOrderBook<u32, u32> = SimpleOrderBook::new();
        assert!(book.asks.levels().is_empty());
        assert!(book.bids.levels().is_empty());
    }
}
