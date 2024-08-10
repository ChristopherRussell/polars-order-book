use order_book::{
    book_side_simple::SimpleBookSide, book_side_tracked::BookSideWithTopNTracking,
    book_side_tracked_basic::BookSideWithBasicTracking,
};
use order_book_core::book_side::BookSide;
use order_book_core::book_side_ops::PricePointMutationOps;
use order_book_core::price_level::{self, AskPrice, BidPrice, PriceLevel};

const PRICE_AND_LEVELS: [(u32, u32); 4] = [(1, 100), (2, 100), (3, 101), (4, 98)];
const BID_SORTED_PRICE_AND_LEVELS: [(u32, u32); 4] = [(4, 98), (3, 101), (2, 100), (1, 100)];
const ASK_SORTED_PRICE_AND_LEVELS: [(u32, u32); 4] = [(1, 100), (2, 100), (3, 101), (4, 98)];

fn create_book_side_with_orders<Px, BS>() -> BS
where
    Px: price_level::Price<PriceType = u32> + From<u32>,
    BS: BookSide<Px, u32> + PricePointMutationOps<Px, u32> + Default,
{
    let mut book_side = BS::default();
    for (price, qty) in PRICE_AND_LEVELS.iter() {
        book_side.add_qty(Px::from(*price), *qty);
    }
    book_side
}

#[test]
fn test_new() {
    fn test_new_for_book_side<Px, BS>()
    where
        Px: price_level::Price<PriceType = u32> + From<u32>,
        BS: BookSide<Px, u32> + Default,
    {
        let book_side: BS = BS::default();
        assert_eq!(book_side.levels().len(), 0);
    }

    test_new_for_book_side::<BidPrice<u32>, SimpleBookSide<BidPrice<u32>, u32>>();
    test_new_for_book_side::<AskPrice<u32>, SimpleBookSide<AskPrice<u32>, u32>>();
    test_new_for_book_side::<BidPrice<u32>, BookSideWithTopNTracking<BidPrice<u32>, u32, 5>>();
    test_new_for_book_side::<AskPrice<u32>, BookSideWithTopNTracking<AskPrice<u32>, u32, 5>>();
    test_new_for_book_side::<BidPrice<u32>, BookSideWithBasicTracking<BidPrice<u32>, u32>>();
    test_new_for_book_side::<AskPrice<u32>, BookSideWithBasicTracking<AskPrice<u32>, u32>>();
}

#[test]
fn test_add_qty_to_empty_book() {
    fn test_add_qty_to_empty_book_for_side<Px, BS>()
    where
        Px: price_level::Price<PriceType = u32> + From<u32>,
        BS: BookSide<Px, u32> + PricePointMutationOps<Px, u32> + Default,
    {
        let qty = 5;
        let price = Px::from(100);
        let mut book_side = BS::default();
        book_side.add_qty(price, qty);
        assert_qty_added(&book_side, price, qty, 0, 0);
    }

    test_add_qty_to_empty_book_for_side::<BidPrice<u32>, SimpleBookSide<BidPrice<u32>, u32>>();
    test_add_qty_to_empty_book_for_side::<AskPrice<u32>, SimpleBookSide<AskPrice<u32>, u32>>();
    test_add_qty_to_empty_book_for_side::<
        BidPrice<u32>,
        BookSideWithTopNTracking<BidPrice<u32>, u32, 5>,
    >();
    test_add_qty_to_empty_book_for_side::<
        AskPrice<u32>,
        BookSideWithTopNTracking<AskPrice<u32>, u32, 5>,
    >();
    test_add_qty_to_empty_book_for_side::<
        BidPrice<u32>,
        BookSideWithBasicTracking<BidPrice<u32>, u32>,
    >();
    test_add_qty_to_empty_book_for_side::<
        AskPrice<u32>,
        BookSideWithBasicTracking<AskPrice<u32>, u32>,
    >();
}

#[test]
fn test_add_qty() {
    fn test_add_qty_for_side<Px, BS>()
    where
        Px: price_level::Price<PriceType = u32> + From<u32>,
        BS: BookSide<Px, u32> + PricePointMutationOps<Px, u32> + Default,
    {
        for (price, qty) in PRICE_AND_LEVELS.into_iter() {
            let mut book_side: BS = create_book_side_with_orders();
            let price = Px::from(price);
            let num_levels_before = book_side.levels().len();
            let qty_before = book_side.levels().get(&price).map_or(0, |&q| q);
            book_side.add_qty(price, qty);
            assert_qty_added(&book_side, price, qty, qty_before, num_levels_before);
        }
    }

    test_add_qty_for_side::<BidPrice<u32>, SimpleBookSide<BidPrice<u32>, u32>>();
    test_add_qty_for_side::<AskPrice<u32>, SimpleBookSide<AskPrice<u32>, u32>>();
    test_add_qty_for_side::<BidPrice<u32>, BookSideWithTopNTracking<BidPrice<u32>, u32, 5>>();
    test_add_qty_for_side::<AskPrice<u32>, BookSideWithTopNTracking<AskPrice<u32>, u32, 5>>();
    test_add_qty_for_side::<BidPrice<u32>, BookSideWithBasicTracking<BidPrice<u32>, u32>>();
    test_add_qty_for_side::<AskPrice<u32>, BookSideWithBasicTracking<AskPrice<u32>, u32>>();
}

// Common assertion helper
fn assert_qty_added<Px: price_level::Price>(
    book_side: &impl BookSide<Px, u32>,
    price: Px,
    qty: u32,
    qty_before: u32,
    num_levels_before: usize,
) {
    let new_level_created = qty_before == 0;
    assert_eq!(
        book_side.levels().len(),
        num_levels_before + new_level_created as usize
    );
    let new_qty = book_side.levels().get(&price).expect("Level not found");
    assert_eq!(*new_qty, qty_before + qty);
}

#[test]
fn test_delete_qty() {
    fn test_delete_qty_for_side<Px, BS>()
    where
        Px: price_level::Price<PriceType = u32> + From<u32>,
        BS: BookSide<Px, u32> + PricePointMutationOps<Px, u32> + Default,
    {
        let mut book_side = BS::default();
        let (price, qty) = (Px::from(100), 10);
        book_side.add_qty(price, qty);
        book_side.delete_qty(price, qty).unwrap();
        assert_eq!(book_side.levels().len(), 0);
    }

    test_delete_qty_for_side::<BidPrice<u32>, SimpleBookSide<BidPrice<u32>, u32>>();
    test_delete_qty_for_side::<AskPrice<u32>, SimpleBookSide<AskPrice<u32>, u32>>();
    test_delete_qty_for_side::<BidPrice<u32>, BookSideWithTopNTracking<BidPrice<u32>, u32, 5>>();
    test_delete_qty_for_side::<AskPrice<u32>, BookSideWithTopNTracking<AskPrice<u32>, u32, 5>>();
    test_delete_qty_for_side::<BidPrice<u32>, BookSideWithBasicTracking<BidPrice<u32>, u32>>();
    test_delete_qty_for_side::<AskPrice<u32>, BookSideWithBasicTracking<AskPrice<u32>, u32>>();
}

fn _test_get_nth_best_level_for_sides<BidSide, AskSide>()
where
    BidSide: BookSide<BidPrice<u32>, u32> + PricePointMutationOps<BidPrice<u32>, u32> + Default,
    AskSide: BookSide<AskPrice<u32>, u32> + PricePointMutationOps<AskPrice<u32>, u32> + Default,
{
    let mut bid_side: BidSide = create_book_side_with_orders::<BidPrice<u32>, BidSide>();
    let mut ask_side: AskSide = create_book_side_with_orders::<AskPrice<u32>, AskSide>();

    for (level, ((expected_bid_px, expected_bid_qty), (expected_ask_px, expected_ask_qty))) in
        BID_SORTED_PRICE_AND_LEVELS
            .into_iter()
            .zip(ASK_SORTED_PRICE_AND_LEVELS.into_iter())
            .enumerate()
    {
        assert_eq!(
            bid_side.nth_best_level(level),
            Some(PriceLevel {
                price: expected_bid_px.into(),
                qty: expected_bid_qty
            })
        );
        assert_eq!(
            ask_side.nth_best_level(level),
            Some(PriceLevel {
                price: expected_ask_px.into(),
                qty: expected_ask_qty
            })
        );
    }

    // Non-existent level
    assert_eq!(bid_side.nth_best_level(4), None);
    assert_eq!(ask_side.nth_best_level(4), None);

    bid_side.delete_qty(3.into(), 101).unwrap();
    ask_side.delete_qty(3.into(), 101).unwrap();

    for (level, (expected_price, expected_qty)) in
        [(4, 98), (2, 100), (1, 100)].into_iter().enumerate()
    {
        assert_eq!(
            bid_side.nth_best_level(level),
            Some(PriceLevel {
                price: expected_price.into(),
                qty: expected_qty
            })
        );
        assert_eq!(
            ask_side.nth_best_level(2 - level),
            Some(PriceLevel {
                price: expected_price.into(),
                qty: expected_qty
            })
        );
    }

    // Non-existent level
    assert_eq!(bid_side.nth_best_level(3), None);
    assert_eq!(ask_side.nth_best_level(3), None);

    bid_side.delete_qty(1.into(), 100).unwrap();
    ask_side.delete_qty(1.into(), 100).unwrap();

    for (level, (expected_price, expected_qty)) in [(4, 98), (2, 100)].into_iter().enumerate() {
        assert_eq!(
            bid_side.nth_best_level(level),
            Some(PriceLevel {
                price: expected_price.into(),
                qty: expected_qty
            })
        );
        assert_eq!(
            ask_side.nth_best_level(1 - level),
            Some(PriceLevel {
                price: expected_price.into(),
                qty: expected_qty
            })
        );
    }

    bid_side.delete_qty(4.into(), 98).unwrap();
    ask_side.delete_qty(4.into(), 98).unwrap();

    assert_eq!(
        bid_side.nth_best_level(0),
        Some(PriceLevel {
            price: 2.into(),
            qty: 100
        })
    );
    assert_eq!(
        ask_side.nth_best_level(0),
        Some(PriceLevel {
            price: 2.into(),
            qty: 100
        })
    );

    assert_eq!(bid_side.nth_best_level(1), None);
    assert_eq!(ask_side.nth_best_level(1), None);

    bid_side.delete_qty(2.into(), 100).unwrap();
    ask_side.delete_qty(2.into(), 100).unwrap();

    assert_eq!(bid_side.nth_best_level(0), None);
    assert_eq!(ask_side.nth_best_level(0), None);
}

#[test]
fn test_get_nth_best_level() {
    _test_get_nth_best_level_for_sides::<
        SimpleBookSide<BidPrice<u32>, u32>,
        SimpleBookSide<AskPrice<u32>, u32>,
    >();
    _test_get_nth_best_level_for_sides::<
        BookSideWithTopNTracking<BidPrice<u32>, u32, 5>,
        BookSideWithTopNTracking<AskPrice<u32>, u32, 5>,
    >();
    _test_get_nth_best_level_for_sides::<
        BookSideWithBasicTracking<BidPrice<u32>, u32>,
        BookSideWithBasicTracking<AskPrice<u32>, u32>,
    >();
}
