use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::Hash;

use num::Num;

use crate::book_side::{BookSide, DeleteLevelType, FoundLevelType};
use crate::book_side_ops::{BookSideOps, BookSideOpsError};
use crate::price_level::PriceLevel;
use crate::top_n_levels::NLevels;
use tracing::{debug, instrument};

pub struct BookSideWithTopNTracking<Price, Qty, const N: usize> {
    book_side: BookSide<Price, Qty>,
    top_n_levels: NLevels<Price, Qty, N>,
}

impl<Price: Debug, Qty: Debug, const N: usize> Debug for BookSideWithTopNTracking<Price, Qty, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BookSideWithTop-{}Tracking {{ top-levels: {:?} }}",
            N, self.top_n_levels
        )
    }
}
impl<Price: Ord + Hash + Copy + Debug, Qty: Num + Ord + Debug + Copy, const N: usize>
    BookSideWithTopNTracking<Price, Qty, N>
{
    pub fn new(is_bid: bool) -> Self {
        BookSideWithTopNTracking {
            book_side: BookSide::new(is_bid),
            top_n_levels: NLevels::new(),
        }
    }

    pub fn get_nth_best_level(&self) -> Option<PriceLevel<Price, Qty>> {
        self.book_side.get_nth_best_level(N-1)
    }

    pub fn get_level(&self, price: Price) -> Option<&PriceLevel<Price, Qty>> {
        self.book_side.get_level(price)
    }

    pub fn top_n(&self) -> &[Option<PriceLevel<Price, Qty>>; N] {
        &self.top_n_levels.levels
    }
}

impl<
        Price: Debug + Eq + Ord + Copy + Hash,
        Qty: Debug + Ord + Clone + Copy + Num,
        const N: usize,
    > BookSideOps<Price, Qty> for BookSideWithTopNTracking<Price, Qty, N>
{
    #[instrument]
    fn add_qty(&mut self, price: Price, qty: Qty) -> (FoundLevelType, PriceLevel<Price, Qty>) {
        let (
            found_level_type,
            PriceLevel {
                price: added_price,
                qty: added_qty, // TODO: this name is deceptive, it's total qty not the change
            },
        ) = self.book_side.add_qty(price, qty);

        match (
            found_level_type,
            self.book_side.is_bid,
            self.top_n_levels.worst_price.map(|px| added_price.cmp(&px)),
        ) {
            // Ignore bid below worst tracked price or ask above worst tracked price
            (_, true, Some(Ordering::Less)) | (_, false, Some(Ordering::Greater)) => {
                debug!(
                    "Ignoring price worse than worst tracked price. Price: {:?}, Worst Price: {:?}, Is Bid: {:?}",
                    added_price, self.top_n_levels.worst_price, self.book_side.is_bid
                );
            }
            // Adding qty to existing tracked price
            (FoundLevelType::Existing, _, _) => {
                self.top_n_levels.update_qty(added_price, added_qty);
                debug!(
                    "Updated qty at tracked level. Price: {:?}, Qty: {:?}",
                    added_price, added_qty
                )
            }
            // Insert new top_n bid
            (FoundLevelType::New, true, _) => {
                self.top_n_levels.try_insert_sort(PriceLevel {
                    price: added_price,
                    qty: added_qty,
                });
                debug!(
                    "Inserted new top_n bid. Price: {:?}, Qty: {:?}",
                    added_price, added_qty
                )
            }
            // Insert new top_n ask
            (FoundLevelType::New, false, _) => {
                self.top_n_levels.insert_sort_reversed(PriceLevel {
                    price: added_price,
                    qty: added_qty,
                });
                debug!(
                    "Inserted new top_n ask. Price: {:?}, Qty: {:?}",
                    added_price, added_qty
                )
            }
        }
        (
            found_level_type,
            PriceLevel {
                price: added_price,
                qty: added_qty,
            },
        )
    }

    #[instrument]
    fn delete_qty(
        &mut self,
        price: Price,
        qty: Qty,
    ) -> Result<(DeleteLevelType, PriceLevel<Price, Qty>), BookSideOpsError> {
        let (delete_type, level) = self.book_side.delete_qty(price, qty)?;
        match (
            delete_type,
            self.book_side.is_bid,
            self.top_n_levels.worst_price.map(|px| px.cmp(&level.price)),
        ) {
            // Ignore delete at a level below worst tracked price
            (_, true, Some(Ordering::Greater)) | (_, false, Some(Ordering::Less)) => {}
            // Quantity decreased at a tracked level
            (DeleteLevelType::QuantityDecreased, _, _) => {
                self.top_n_levels.update_qty(level.price, level.qty);
                debug!(
                    "Updated qty at tracked level. Price: {:?}, Qty: {:?}",
                    level.price, level.qty
                );
            }
            // Tracked level delete, find next best level and replace
            (DeleteLevelType::Deleted, _, _) => {
                let best_untracked_level = self.get_nth_best_level();
                self.top_n_levels
                    .replace_sort(level.price, best_untracked_level);
                debug!(
                    "Replaced tracked level with next best level. Price: {:?}, Qty: {:?}",
                    level.price, level.qty
                );
            }
        }
        Ok((delete_type, level))
    }
}

impl<Price: Ord + Hash + Copy + Debug, Qty: Num + Ord + Debug + Copy, const N: usize>
    BookSideWithTopNTracking<Price, Qty, N>
{
    pub fn best_price(&self) -> Option<Price> {
        self.top_n_levels.best_price()
    }
    pub fn best_price_qty(&self) -> Option<Qty> {
        self.top_n_levels.best_price_qty()
    }
}
#[cfg(test)]
mod tests {
    use tracing::Level;

    use super::*;

    fn create_books() -> (
        BookSideWithTopNTracking<i32, i32, 1>,
        BookSideWithTopNTracking<i32, i32, 2>,
        BookSideWithTopNTracking<i32, i32, 3>,
        BookSideWithTopNTracking<i32, i32, 1>,
        BookSideWithTopNTracking<i32, i32, 2>,
        BookSideWithTopNTracking<i32, i32, 3>,
    ) {
        (
            BookSideWithTopNTracking::new(true),
            BookSideWithTopNTracking::new(true),
            BookSideWithTopNTracking::new(true),
            BookSideWithTopNTracking::new(false),
            BookSideWithTopNTracking::new(false),
            BookSideWithTopNTracking::new(false),
        )
    }

    // Macro to assert the top_n values for all book sides in a tuple
    macro_rules! assert_top_n {
        ($expected:expr, $books:expr) => {
            let (ref book_side_1, ref book_side_2, ref book_side_3) = $books;
            assert_eq!(book_side_1.top_n(), &$expected[..1]);
            assert_eq!(book_side_2.top_n(), &$expected[..2]);
            assert_eq!(book_side_3.top_n(), &$expected[..3]);

            let best_price = $expected[0].map(|pl| pl.price);
            assert_eq!(book_side_1.best_price(), best_price);
            assert_eq!(book_side_2.best_price(), best_price);
            assert_eq!(book_side_3.best_price(), best_price);

            let best_price_qty = $expected[0].map(|pl| pl.qty);
            assert_eq!(book_side_1.best_price_qty(), best_price_qty);
            assert_eq!(book_side_2.best_price_qty(), best_price_qty);
            assert_eq!(book_side_3.best_price_qty(), best_price_qty);
        };
    }

    // Macro to assert the top_n values for all book sides in a tuple
    macro_rules! assert_top_n_bids {
        ($expected:expr, $books:expr) => {
            let (ref book_side_1, ref book_side_2, ref book_side_3, _, _, _) = $books;
            assert_top_n!($expected, (book_side_1, book_side_2, book_side_3));
        };
    }

    macro_rules! assert_top_n_asks {
        ($expected:expr, $books:expr) => {
            let (_, _, _, ref book_side_1, ref book_side_2, ref book_side_3) = $books;
            assert_top_n!($expected, (book_side_1, book_side_2, book_side_3));
        };
    }

    macro_rules! add_qty {
        ($price:expr, $qty:expr, $books:expr) => {
            let (
                ref mut bid_side_1,
                ref mut bid_side_2,
                ref mut bid_side_3,
                ref mut ask_side_1,
                ref mut ask_side_2,
                ref mut ask_side_3,
            ) = $books;
            bid_side_1.add_qty($price, $qty);
            bid_side_2.add_qty($price, $qty);
            bid_side_3.add_qty($price, $qty);
            ask_side_1.add_qty($price, $qty);
            ask_side_2.add_qty($price, $qty);
            ask_side_3.add_qty($price, $qty);
        };
    }

    macro_rules! delete_qty {
        ($price:expr, $qty:expr, $books:expr) => {
            let (
                ref mut bid_side_1,
                ref mut bid_side_2,
                ref mut bid_side_3,
                ref mut ask_side_1,
                ref mut ask_side_2,
                ref mut ask_side_3,
            ) = $books;
            bid_side_1.delete_qty($price, $qty).unwrap();
            bid_side_2.delete_qty($price, $qty).unwrap();
            bid_side_3.delete_qty($price, $qty).unwrap();
            ask_side_1.delete_qty($price, $qty).unwrap();
            ask_side_2.delete_qty($price, $qty).unwrap();
            ask_side_3.delete_qty($price, $qty).unwrap();
        };
    }

    #[test]
    fn test_add_more_levels_than_tracked() {
        let mut book_sides = create_books();
        let prices = [400, 100, 200, 300, 400, 100];
        let qtys = [19, 6, 20, 30, 21, 4];
        for (price, qty) in prices.iter().zip(qtys.iter()) {
            add_qty!(*price, *qty, book_sides);
        }

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: 400,
                qty: 40,
            }),
            Some(PriceLevel {
                price: 300,
                qty: 30,
            }),
            Some(PriceLevel {
                price: 200,
                qty: 20,
            }),
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: 100,
                qty: 10,
            }),
            Some(PriceLevel {
                price: 200,
                qty: 20,
            }),
            Some(PriceLevel {
                price: 300,
                qty: 30,
            }),
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);
    }

    #[test]
    fn test_delete_qty() {
        let mut book_sides = create_books();
        add_qty!(100, 10, book_sides);
        let expected_top_n = [
            Some(PriceLevel {
                price: 100,
                qty: 10,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);

        delete_qty!(100, 10, book_sides);
        let expected_top_n = [None, None, None];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);
    }

    #[test]
    fn test_best_price_after_add_better() {
        let mut book_sides = create_books();
        add_qty!(100, 10, book_sides);
        let expected_top_n = [
            Some(PriceLevel {
                price: 100,
                qty: 10,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);

        add_qty!(101, 20, book_sides);
        let expected_top_n_bids = [
            Some(PriceLevel {
                price: 101,
                qty: 20,
            }),
            Some(PriceLevel {
                price: 100,
                qty: 10,
            }),
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: 100,
                qty: 10,
            }),
            Some(PriceLevel {
                price: 101,
                qty: 20,
            }),
            None,
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);
    }

    #[test]
    fn test_best_price_modify_quantity() {
        let mut book_sides = create_books();
        add_qty!(100, 10, book_sides);
        let expected_top_n = [
            Some(PriceLevel {
                price: 100,
                qty: 10,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);

        add_qty!(100, 20, book_sides);
        let expected_top_n = [
            Some(PriceLevel {
                price: 100,
                qty: 30,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);

        delete_qty!(100, 15, book_sides);
        let expected_top_n = [
            Some(PriceLevel {
                price: 100,
                qty: 15,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);

        delete_qty!(100, 15, book_sides);
        let expected_top_n = [None, None, None];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);
    }

    #[test]
    fn test_modify_price() {
        let mut book_sides = create_books();
        add_qty!(100, 10, book_sides);
        let expected_top_n = [
            Some(PriceLevel {
                price: 100,
                qty: 10,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);

        delete_qty!(100, 10, book_sides);
        add_qty!(101, 20, book_sides);
        let expected_top_n = [
            Some(PriceLevel {
                price: 101,
                qty: 20,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);

        delete_qty!(101, 20, book_sides);
        add_qty!(100, 15, book_sides);
        let expected_top_n = [
            Some(PriceLevel {
                price: 100,
                qty: 15,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);
    }

    #[test]
    fn test_book_side_with_cyclic_modify_price() {
        let mut book_sides = create_books();
        add_qty!(100, 10, book_sides);
        delete_qty!(100, 10, book_sides);
        add_qty!(101, 11, book_sides);
        let expected_top_n = [
            Some(PriceLevel {
                price: 101,
                qty: 11,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);

        delete_qty!(101, 11, book_sides);
        let expected_top_n = [None, None, None];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);

        add_qty!(100, 12, book_sides);
        let expected_top_n = [
            Some(PriceLevel {
                price: 100,
                qty: 12,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);

        delete_qty!(100, 12, book_sides);
        let expected_top_n = [None, None, None];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);

        add_qty!(102, 13, book_sides);
        let expected_top_n = [
            Some(PriceLevel {
                price: 102,
                qty: 13,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n, book_sides);
        assert_top_n_asks!(expected_top_n, book_sides);
    }

    #[test]
    fn test_full_book_side_with_cyclic_modify_price() {
        tracing_subscriber::fmt()
            .pretty()
            .with_max_level(Level::TRACE)
            .with_test_writer()
            .init();
        let mut book_sides = create_books();
        add_qty!(100, 10, book_sides);
        add_qty!(101, 11, book_sides);
        add_qty!(102, 12, book_sides);
        add_qty!(103, 13, book_sides);
        add_qty!(104, 14, book_sides);
        add_qty!(105, 15, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: 105,
                qty: 15,
            }),
            Some(PriceLevel {
                price: 104,
                qty: 14,
            }),
            Some(PriceLevel {
                price: 103,
                qty: 13,
            }),
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: 100,
                qty: 10,
            }),
            Some(PriceLevel {
                price: 101,
                qty: 11,
            }),
            Some(PriceLevel {
                price: 102,
                qty: 12,
            }),
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        delete_qty!(100, 10, book_sides);

        let expected_top_n_asks = [
            Some(PriceLevel {
                price: 101,
                qty: 11,
            }),
            Some(PriceLevel {
                price: 102,
                qty: 12,
            }),
            Some(PriceLevel {
                price: 103,
                qty: 13,
            }),
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        add_qty!(99, 9, book_sides);

        let expected_top_n_asks = [
            Some(PriceLevel { price: 99, qty: 9 }),
            Some(PriceLevel {
                price: 101,
                qty: 11,
            }),
            Some(PriceLevel {
                price: 102,
                qty: 12,
            }),
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        delete_qty!(105, 15, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: 104,
                qty: 14,
            }),
            Some(PriceLevel {
                price: 103,
                qty: 13,
            }),
            Some(PriceLevel {
                price: 102,
                qty: 12,
            }),
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        add_qty!(106, 16, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: 106,
                qty: 16,
            }),
            Some(PriceLevel {
                price: 104,
                qty: 14,
            }),
            Some(PriceLevel {
                price: 103,
                qty: 13,
            }),
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);
    }
}
