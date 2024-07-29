use std::fmt::Debug;
use std::hash::Hash;

use num::traits::Num;

use crate::book_side_tracked::BookSideWithTopNTracking;
use crate::order_book::BidAskBook;

pub struct OrderBookWithTopNTracking<Price, Qty, const N: usize> {
    pub bids: BookSideWithTopNTracking<Price, Qty, N>,
    pub offers: BookSideWithTopNTracking<Price, Qty, N>,
}

impl<Price: Debug, Qty: Debug, const N: usize> Debug for OrderBookWithTopNTracking<Price, Qty, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "OrderBookWithTopNTracking-{}Tracking {{ Bids: {:?}, Asks: {:?} }}",
            N, self.bids, self.offers
        )
    }
}

impl<Price, Qty, const N: usize> BidAskBook<Price, Qty>
    for OrderBookWithTopNTracking<Price, Qty, N>
{
    type BookSide = BookSideWithTopNTracking<Price, Qty, N>;

    fn book_side(&mut self, is_bid: bool) -> &mut BookSideWithTopNTracking<Price, Qty, N> {
        if is_bid {
            &mut self.bids
        } else {
            &mut self.offers
        }
    }
}

impl<Price: Copy + Debug + Hash + Ord, Qty: Copy + Debug + Num + Ord, const N: usize> Default
    for OrderBookWithTopNTracking<Price, Qty, N>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Price: Copy + Debug + Hash + Ord, Qty: Copy + Debug + Num + Ord, const N: usize>
    OrderBookWithTopNTracking<Price, Qty, N>
{
    pub fn new() -> Self {
        OrderBookWithTopNTracking {
            bids: BookSideWithTopNTracking::new(true),
            offers: BookSideWithTopNTracking::new(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::order_book::PricePointMutationBookOps;

    #[test]
    fn test_add_qty() {
        let price = 100;
        let mut order_book: OrderBookWithTopNTracking<_, _, 1> =
            OrderBookWithTopNTracking::default();
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
        let mut order_book: OrderBookWithTopNTracking<_, _, 1> =
            OrderBookWithTopNTracking::default();
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
        for is_bid in [true, false] {
            let mut order_book: OrderBookWithTopNTracking<_, _, 1> =
                OrderBookWithTopNTracking::default();
            order_book.add_qty(is_bid, 100, 10);
            assert_eq!(order_book.book_side(is_bid).get_level(100).unwrap().qty, 10);
            order_book.modify_qty(is_bid, 100, 10, 100, 20);
            assert_eq!(order_book.book_side(is_bid).get_level(100).unwrap().qty, 20);
        }
    }

    #[test]
    fn test_modify_price() {
        for is_bid in [true, false] {
            let mut order_book: OrderBookWithTopNTracking<_, _, 1> =
                OrderBookWithTopNTracking::default();
            order_book.add_qty(is_bid, 1, 1);
            assert_eq!(order_book.book_side(is_bid).get_level(1).unwrap().qty, 1);
            order_book.modify_qty(is_bid, 1, 1, 2, 2);
            assert_eq!(order_book.book_side(is_bid).get_level(2).unwrap().qty, 2);
            order_book.modify_qty(is_bid, 2, 2, 1, 1);
            assert_eq!(order_book.book_side(is_bid).get_level(1).unwrap().qty, 1);
        }
    }
}
