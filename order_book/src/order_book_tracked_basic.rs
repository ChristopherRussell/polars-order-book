use std::fmt::{Debug, Display};
use std::hash::Hash;

use anyhow::Context;
use num::traits::Num;

use crate::book_side_tracked_basic::BookSideWithBasicTracking;

pub struct OrderBookWithBasicTracking<Price, Qty> {
    bids: BookSideWithBasicTracking<Price, Qty>,
    offers: BookSideWithBasicTracking<Price, Qty>,
}

impl<Price: Copy + Debug + Display + Hash + Ord, Qty: Copy + Debug + Display + Num + Ord> Default
    for OrderBookWithBasicTracking<Price, Qty>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Price: Copy + Debug + Display + Hash + Ord, Qty: Copy + Debug + Display + Num + Ord>
    OrderBookWithBasicTracking<Price, Qty>
{
    pub fn new() -> Self {
        OrderBookWithBasicTracking {
            bids: BookSideWithBasicTracking::new(true),
            offers: BookSideWithBasicTracking::new(false),
        }
    }

    #[inline]
    pub fn book_side(&mut self, is_bid: bool) -> &mut BookSideWithBasicTracking<Price, Qty> {
        if is_bid {
            &mut self.bids
        } else {
            &mut self.offers
        }
    }

    pub fn add_qty(&mut self, is_bid: bool, price: Price, qty: Qty) {
        self.book_side(is_bid).add_qty(price, qty)
    }

    pub fn modify_qty(
        &mut self,
        is_bid: bool,
        prev_price: Price,
        prev_qty: Qty,
        new_price: Price,
        new_qty: Qty,
    ) {
        self.delete_qty(is_bid, prev_price, prev_qty);
        self.add_qty(is_bid, new_price, new_qty);
    }

    pub fn delete_qty(&mut self, is_bid: bool, price: Price, qty: Qty) {
        self.book_side(is_bid)
            .delete_qty(price, qty)
            .with_context(|| {
                format!(
                    "Failed to delete qty from price level: is_bid: {}, price: {}, qty: {}",
                    is_bid, price, qty
                )
            })
            .unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_qty() {
        let price = 100;
        let mut order_book = OrderBookWithBasicTracking::default();
        for is_bid in [true, false].iter() {
            let mut current_qty = 0;
            for _ in 0..10 {
                order_book.add_qty(*is_bid, price, 10);
                current_qty += 10;
                let level = order_book.book_side(*is_bid).get_level(price);
                let level_qty = level.unwrap().qty;
                assert_eq!(level_qty, current_qty);
            }
        }
    }

    #[test]
    fn test_cancel_order() {
        let mut order_book = OrderBookWithBasicTracking::default();
        order_book.add_qty(true, 100, 10);
        assert_eq!(order_book.book_side(true).get_level(100).unwrap().qty, 10);
        order_book.delete_qty(true, 100, 10);
        assert!(order_book.book_side(true).get_level(100).is_none());

        order_book.add_qty(true, 100, 10);
        assert_eq!(order_book.book_side(true).get_level(100).unwrap().qty, 10);
        order_book.delete_qty(true, 100, 5);
        assert_eq!(order_book.book_side(true).get_level(100).unwrap().qty, 5);
        order_book.delete_qty(true, 100, 5);
        assert!(order_book.book_side(true).get_level(100).is_none());
    }

    #[test]
    fn test_modify_qty() {
        for is_bid in vec![true, false] {
            let mut order_book = OrderBookWithBasicTracking::default();
            order_book.add_qty(is_bid, 100, 10);
            assert_eq!(order_book.book_side(is_bid).get_level(100).unwrap().qty, 10);
            order_book.modify_qty(is_bid, 100, 10, 100, 20);
            assert_eq!(order_book.book_side(is_bid).get_level(100).unwrap().qty, 20);
        }
    }

    #[test]
    fn test_modify_price() {
        for is_bid in vec![true, false] {
            let mut order_book = OrderBookWithBasicTracking::default();
            order_book.add_qty(is_bid, 1, 1);
            assert_eq!(order_book.book_side(is_bid).get_level(1).unwrap().qty, 1);
            order_book.modify_qty(is_bid, 1, 1, 2, 2);
            assert_eq!(order_book.book_side(is_bid).get_level(2).unwrap().qty, 2);
            order_book.modify_qty(is_bid, 2, 2, 1, 1);
            assert_eq!(order_book.book_side(is_bid).get_level(1).unwrap().qty, 1);
        }
    }
}
