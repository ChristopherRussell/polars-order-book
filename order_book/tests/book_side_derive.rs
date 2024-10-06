// use order_book::book_side::BookSide;
use hashbrown::HashMap;

use order_book_core::book_side::BookSide;
use order_book_core::price_level::{BidPrice, Price, QuantityLike};
use order_book_derive::BookSide;
#[derive(BookSide)]
struct SimpleBookSide<Px: Price, Qty: QuantityLike> {
    levels: HashMap<Px, Qty>,
}
// debug
impl<Px: Price, Qty: QuantityLike> std::fmt::Debug for SimpleBookSide<Px, Qty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SimpleBookSide {{ levels: {:?} }}", self.levels)
    }
}

#[test]
fn test_book_side_levels() {
    let mut book = SimpleBookSide {
        levels: HashMap::new(),
    };

    // Testing levels()
    let levels_ref = book.levels();
    assert_eq!(levels_ref.len(), 0);

    // Testing levels_mut()
    let levels_mut_ref = book.levels_mut();
    levels_mut_ref.insert(BidPrice(1), 100);

    assert_eq!(book.levels().get(&BidPrice(1)), Some(&100));
}
