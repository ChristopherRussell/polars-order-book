use order_book::order_book_simple::SimpleOrderBook;
use order_book::order_book_tracked::OrderBookWithTopNTracking;
use order_book::order_book_tracked_basic::OrderBookWithBasicTracking;
use order_book_core::order_book::{BidAskBookTestMethods, PricePointMutationBookOps};

fn _test_add_qty<OrderBook: PricePointMutationBookOps<i64, i64> + Default, const IS_BID: bool>() {
    let mut order_book = OrderBook::default();
    let price = 100;
    let mut current_qty = 0;
    for _ in 0..10 {
        current_qty += 10;
        order_book.add_qty(IS_BID, price, 10.into());
        let level_qty = order_book.get_level_qty(IS_BID, price);
        assert_eq!(level_qty, Some(current_qty));
    }
}

#[test]
fn test_add_qty() {
    _test_add_qty::<SimpleOrderBook<i64, i64>, true>();
    _test_add_qty::<SimpleOrderBook<i64, i64>, false>();
    _test_add_qty::<OrderBookWithTopNTracking<i64, i64, 5>, true>();
    _test_add_qty::<OrderBookWithTopNTracking<i64, i64, 5>, false>();
    _test_add_qty::<OrderBookWithBasicTracking<i64, i64>, true>();
    _test_add_qty::<OrderBookWithBasicTracking<i64, i64>, false>();
}

fn _test_cancel_order<
    OrderBook: PricePointMutationBookOps<i64, i64> + Default,
    const IS_BID: bool,
>() {
    let mut order_book = OrderBook::default();
    order_book.add_qty(IS_BID, 100, 10);
    assert_eq!(order_book.get_level_qty(IS_BID, 100), Some(10));
    order_book.delete_qty(IS_BID, 100, 10).unwrap();
}

#[test]
fn test_cancel_order() {
    _test_cancel_order::<SimpleOrderBook<i64, i64>, true>();
    _test_cancel_order::<SimpleOrderBook<i64, i64>, false>();
    _test_cancel_order::<OrderBookWithTopNTracking<i64, i64, 5>, true>();
    _test_cancel_order::<OrderBookWithTopNTracking<i64, i64, 5>, false>();
    _test_cancel_order::<OrderBookWithBasicTracking<i64, i64>, true>();
    _test_cancel_order::<OrderBookWithBasicTracking<i64, i64>, false>();
}

fn _test_modify_qty<
    OrderBook: PricePointMutationBookOps<i64, i64> + Default,
    const IS_BID: bool,
>() {
    let mut order_book = OrderBook::default();
    order_book.add_qty(IS_BID, 100, 10);
    assert_eq!(order_book.get_level_qty(IS_BID, 100), Some(10));

    order_book.modify_qty(IS_BID, 100, 10, 100, 20).unwrap();
    assert_eq!(order_book.get_level_qty(IS_BID, 100), Some(20));

    order_book.add_qty(IS_BID, 100, 10);
    assert_eq!(order_book.get_level_qty(IS_BID, 100), Some(30));

    order_book.modify_qty(IS_BID, 100, 30, 100, 20).unwrap();
    assert_eq!(order_book.get_level_qty(IS_BID, 100), Some(20));
}

#[test]
fn test_modify_qty() {
    _test_modify_qty::<SimpleOrderBook<i64, i64>, true>();
    _test_modify_qty::<SimpleOrderBook<i64, i64>, false>();
    _test_modify_qty::<OrderBookWithTopNTracking<i64, i64, 5>, true>();
    _test_modify_qty::<OrderBookWithTopNTracking<i64, i64, 5>, false>();
    _test_modify_qty::<OrderBookWithBasicTracking<i64, i64>, true>();
    _test_modify_qty::<OrderBookWithBasicTracking<i64, i64>, false>();
}

fn _test_modify_price<
    OrderBook: PricePointMutationBookOps<i64, i64> + Default,
    const IS_BID: bool,
>() {
    let mut order_book = OrderBook::default();
    order_book.add_qty(IS_BID, 100, 10);
    assert_eq!(order_book.get_level_qty(IS_BID, 100), Some(10));

    order_book.modify_qty(IS_BID, 100, 10, 101, 20).unwrap();
    assert_eq!(order_book.get_level_qty(IS_BID, 100), None);
    assert_eq!(order_book.get_level_qty(IS_BID, 101), Some(20));

    order_book.modify_qty(IS_BID, 101, 20, 100, 10).unwrap();
    assert_eq!(order_book.get_level_qty(IS_BID, 101), None);
    assert_eq!(order_book.get_level_qty(IS_BID, 100), Some(10));
}

#[test]
fn test_modify_price() {
    _test_modify_price::<SimpleOrderBook<i64, i64>, true>();
    _test_modify_price::<SimpleOrderBook<i64, i64>, false>();
    _test_modify_price::<OrderBookWithTopNTracking<i64, i64, 5>, true>();
    _test_modify_price::<OrderBookWithTopNTracking<i64, i64, 5>, false>();
    _test_modify_price::<OrderBookWithBasicTracking<i64, i64>, true>();
    _test_modify_price::<OrderBookWithBasicTracking<i64, i64>, false>();
}
