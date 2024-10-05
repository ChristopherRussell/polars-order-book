use std::cmp::Ordering;
use std::fmt::Debug;

use crate::top_n_levels::NLevels;
use hashbrown::HashMap;
use order_book_core::book_side::{BookSide, DeleteLevelType, FoundLevelType};
use order_book_core::book_side_ops::{
    PricePointMutationOps, PricePointMutationOpsError, PricePointSummaryOps,
};
use order_book_core::price_level::{self, PriceLevel, QuantityLike};
use order_book_derive::BookSide;
use tracing::{debug, instrument};

#[derive(BookSide)]
pub struct BookSideWithTopNTracking<Px: price_level::Price, Qty: QuantityLike, const N: usize> {
    levels: HashMap<Px, Qty>,
    top_n_levels: NLevels<Px, Qty, N>,
}

impl<Px: price_level::Price, Qty: QuantityLike, const N: usize>
    BookSideWithTopNTracking<Px, Qty, N>
{
    pub fn new() -> Self {
        BookSideWithTopNTracking {
            levels: HashMap::new(),
            top_n_levels: NLevels::new(),
        }
    }
}

impl<Px: price_level::Price, Qty: QuantityLike, const N: usize> Default
    for BookSideWithTopNTracking<Px, Qty, N>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Px: price_level::Price, Qty: QuantityLike, const N: usize> Debug
    for BookSideWithTopNTracking<Px, Qty, N>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "BookSideWithTop-{}Tracking {{ top-levels: {:?} }}",
            N, self.top_n_levels
        )
    }
}

impl<Px: price_level::Price, Qty: QuantityLike, const N: usize>
    BookSideWithTopNTracking<Px, Qty, N>
{
    pub fn top_n(&self) -> &[Option<PriceLevel<Px, Qty>>; N] {
        &self.top_n_levels.levels
    }
}

impl<Px: price_level::Price, Qty: QuantityLike, const N: usize> PricePointMutationOps<Px, Qty>
    for BookSideWithTopNTracking<Px, Qty, N>
{
    #[instrument]
    fn add_qty(&mut self, price: Px, qty: Qty) -> FoundLevelType<Qty> {
        let found_level_type = self.find_or_create_level_and_add_qty(price, qty);
        if N == 1 {
            debug!(
                "CASEN N=1: Added qty to book_side. Px: {:?}, Qty: {:?}, {:?}, {:?}",
                price,
                qty,
                self.levels().get(&price),
                self.top_n_levels
            )
        }

        match (
            found_level_type,
            self.top_n_levels.worst_price.map(|px| price.cmp(&px)),
        ) {
            // Ignore price below worst tracked price
            (_, Some(Ordering::Less)) => {
                debug!(
                    "Ignoring price worse than worst tracked price. Px: {:?}, Worst Px: {:?}",
                    price, self.top_n_levels.worst_price
                );
            }
            // Adding qty to existing tracked price
            (FoundLevelType::Existing(new_qty), Some(Ordering::Equal)) => {
                if let Some(px_level) = self
                    .top_n_levels
                    .levels
                    .last_mut()
                    .expect("There is at least one element")
                {
                    px_level.qty = new_qty;
                }
                debug!(
                    "Updated qty at worst tracked level. Px: {:?}, New Qty: {:?}, Added Qty: {:?}",
                    price, new_qty, qty
                )
            }
            // Adding qty to existing tracked price - note if worst_price is None then existing price must be tracked
            (FoundLevelType::Existing(new_qty), _) => {
                self.top_n_levels.update_qty(price, new_qty);
                debug!(
                    "Updated qty at tracked level. Px: {:?}, New Qty: {:?}, Added Qty: {:?}",
                    price, new_qty, qty
                )
            }
            // Insert new top_n bid
            (FoundLevelType::New(_), Some(Ordering::Greater) | None) => {
                self.top_n_levels.insert_sort(PriceLevel { price, qty });
                debug!(
                    "Inserted new top_n {}. Px: {:?}, Qty: {:?}",
                    Px::SIDE_NAME,
                    price,
                    qty
                )
            }
            (FoundLevelType::New(_), Some(Ordering::Equal)) => {
                unreachable!("Should not have found a new level at worst price - an existing level")
            }
        }
        found_level_type
    }

    #[instrument]
    fn delete_qty(
        &mut self,
        price: Px,
        qty: Qty,
    ) -> Result<DeleteLevelType<Qty>, PricePointMutationOpsError> {
        let delete_type = self.remove_qty_from_level_and_maybe_delete(price, qty)?;
        match (
            delete_type,
            self.top_n_levels.worst_price.map(|px| px.cmp(&price)),
        ) {
            // Ignore delete at a level below worst tracked price
            (_, Some(Ordering::Greater)) => {}

            // Quantity decreased at a tracked level
            (DeleteLevelType::QuantityDecreased(new_qty), _) => {
                self.top_n_levels.update_qty(price, new_qty);
                debug!(
                    "Updated qty at tracked level. Px: {:?}, Qty: {:?}",
                    price, new_qty
                );
            }
            // Tracked level delete, find next best level and replace
            (DeleteLevelType::Deleted, _) => {
                let best_untracked_level = self.nth_best_level(N - 1);
                self.top_n_levels.replace_sort(price, best_untracked_level);
                debug!(
                    "Replaced tracked level at price {:?} with next best level: {:?}",
                    price, best_untracked_level
                );
            }
        }
        Ok(delete_type)
    }
}

impl<Px: price_level::Price, Qty: QuantityLike, const N: usize>
    BookSideWithTopNTracking<Px, Qty, N>
{
    pub fn best_price(&self) -> Option<Px> {
        self.top_n_levels.best_price()
    }
    pub fn best_price_qty(&self) -> Option<Qty> {
        self.top_n_levels.best_price_qty()
    }
}
impl<Px: price_level::Price, Qty: QuantityLike, const N: usize> PricePointSummaryOps<Px, Qty>
    for BookSideWithTopNTracking<Px, Qty, N>
{
    fn set_level(&mut self, price: Px, new_qty: Qty) {
        if new_qty.is_zero() {
            self.levels.remove(&price);
            match self.top_n_levels.worst_price.map(|px| price.cmp(&px)) {
                Some(Ordering::Greater | Ordering::Equal) => {
                    let best_untracked_level = self.nth_best_level(N - 1);
                    self.top_n_levels.replace_sort(price, best_untracked_level);
                    debug!(
                        "Removed tracked level at price {:?} and replaced with {:?}",
                        price, best_untracked_level
                    );
                }
                None => {
                    self.top_n_levels.replace_sort(price, None);
                    debug!("Removed tracked level at price {:?}, no replacement", price);
                }
                Some(Ordering::Less) => {
                    debug!("Removed untracked level at price {:?}", price);
                }
            }
        } else {
            let found_level_type = self.find_or_create_level_and_set_qty(price, new_qty);
            match (
                found_level_type,
                self.top_n_levels.worst_price.map(|px| price.cmp(&px)),
            ) {
                (FoundLevelType::Existing(_), Some(Ordering::Greater) | None) => {
                    self.top_n_levels.update_qty(price, new_qty);
                    debug!(
                        "Updated existing tracked level. Price: {:?}, New Qty: {:?}",
                        price, new_qty
                    );
                }
                (FoundLevelType::Existing(_), Some(Ordering::Equal)) => {
                    if let Some(px_level) = self
                        .top_n_levels
                        .levels
                        .last_mut()
                        .expect("There is at least one element")
                    {
                        px_level.qty = new_qty;
                    }
                    debug!(
                        "Updated qty at worst tracked level. Price: {:?}, New Qty: {:?}",
                        price, new_qty
                    );
                }
                (FoundLevelType::New(_), Some(Ordering::Greater) | None) => {
                    self.top_n_levels.insert_sort(PriceLevel {
                        price,
                        qty: new_qty,
                    });
                    debug!(
                        "Inserted new tracked level. Price: {:?}, Qty: {:?}",
                        price, new_qty
                    );
                }
                _ => {
                    debug!(
                        "Ignored update for untracked level. Price: {:?}, New Qty: {:?}",
                        price, new_qty
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use order_book_core::price_level::{AskPrice, BidPrice};
    use tracing::Level;

    use super::*;

    #[allow(clippy::type_complexity)]
    fn create_books() -> (
        BookSideWithTopNTracking<BidPrice<i32>, i32, 1>,
        BookSideWithTopNTracking<BidPrice<i32>, i32, 2>,
        BookSideWithTopNTracking<BidPrice<i32>, i32, 3>,
        BookSideWithTopNTracking<AskPrice<i32>, i32, 1>,
        BookSideWithTopNTracking<AskPrice<i32>, i32, 2>,
        BookSideWithTopNTracking<AskPrice<i32>, i32, 3>,
    ) {
        (
            BookSideWithTopNTracking::new(),
            BookSideWithTopNTracking::new(),
            BookSideWithTopNTracking::new(),
            BookSideWithTopNTracking::new(),
            BookSideWithTopNTracking::new(),
            BookSideWithTopNTracking::new(),
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
            assert_eq!(book_side_1.best_price(), best_price.into());
            assert_eq!(book_side_2.best_price(), best_price.into());
            assert_eq!(book_side_3.best_price(), best_price.into());

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
            bid_side_1.add_qty($price.into(), $qty);
            bid_side_2.add_qty($price.into(), $qty);
            bid_side_3.add_qty($price.into(), $qty);
            ask_side_1.add_qty($price.into(), $qty);
            ask_side_2.add_qty($price.into(), $qty);
            ask_side_3.add_qty($price.into(), $qty);
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
            bid_side_1.delete_qty($price.into(), $qty).unwrap();
            bid_side_2.delete_qty($price.into(), $qty).unwrap();
            bid_side_3.delete_qty($price.into(), $qty).unwrap();
            ask_side_1.delete_qty($price.into(), $qty).unwrap();
            ask_side_2.delete_qty($price.into(), $qty).unwrap();
            ask_side_3.delete_qty($price.into(), $qty).unwrap();
        };
    }

    macro_rules! set_level {
        ($price:expr, $qty:expr, $books:expr) => {
            let (
                ref mut bid_side_1,
                ref mut bid_side_2,
                ref mut bid_side_3,
                ref mut ask_side_1,
                ref mut ask_side_2,
                ref mut ask_side_3,
            ) = $books;
            bid_side_1.set_level($price.into(), $qty);
            bid_side_2.set_level($price.into(), $qty);
            bid_side_3.set_level($price.into(), $qty);
            ask_side_1.set_level($price.into(), $qty);
            ask_side_2.set_level($price.into(), $qty);
            ask_side_3.set_level($price.into(), $qty);
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
                price: BidPrice(400),
                qty: 40,
            }),
            Some(PriceLevel {
                price: BidPrice(300),
                qty: 30,
            }),
            Some(PriceLevel {
                price: BidPrice(200),
                qty: 20,
            }),
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 10,
            }),
            Some(PriceLevel {
                price: AskPrice(200),
                qty: 20,
            }),
            Some(PriceLevel {
                price: AskPrice(300),
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
        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(100),
                qty: 10,
            }),
            None,
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 10,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        add_qty!(200, 11, book_sides);
        delete_qty!(200, 11, book_sides);
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        delete_qty!(100, 10, book_sides);
        let expected_top_n_bids = [None, None, None];
        let expected_top_n_asks = [None, None, None];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);
    }

    #[test]
    fn test_best_price_after_add_better() {
        let mut book_sides = create_books();
        add_qty!(100, 10, book_sides);
        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(100),
                qty: 10,
            }),
            None,
            None,
        ];

        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 10,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        add_qty!(101, 20, book_sides);
        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(101),
                qty: 20,
            }),
            Some(PriceLevel {
                price: BidPrice(100),
                qty: 10,
            }),
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 10,
            }),
            Some(PriceLevel {
                price: AskPrice(101),
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
        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(100),
                qty: 10,
            }),
            None,
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 10,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        add_qty!(100, 20, book_sides);
        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(100),
                qty: 30,
            }),
            None,
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 30,
            }),
            None,
            None,
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        delete_qty!(100, 15, book_sides);
        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(100),
                qty: 15,
            }),
            None,
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 15,
            }),
            None,
            None,
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        delete_qty!(100, 15, book_sides);
        let expected_top_n_bids = [None, None, None];
        let expected_top_n_asks = [None, None, None];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);
    }

    #[test]
    fn test_modify_price() {
        let mut book_sides = create_books();
        add_qty!(100, 10, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(100),
                qty: 10,
            }),
            None,
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 10,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        delete_qty!(100, 10, book_sides);
        add_qty!(101, 20, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(101),
                qty: 20,
            }),
            None,
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(101),
                qty: 20,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        delete_qty!(101, 20, book_sides);
        add_qty!(100, 15, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(100),
                qty: 15,
            }),
            None,
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 15,
            }),
            None,
            None,
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);
    }

    #[test]
    fn test_book_side_with_cyclic_modify_price() {
        let mut book_sides = create_books();
        add_qty!(100, 10, book_sides);
        delete_qty!(100, 10, book_sides);
        add_qty!(101, 11, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(101),
                qty: 11,
            }),
            None,
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(101),
                qty: 11,
            }),
            None,
            None,
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        delete_qty!(101, 11, book_sides);

        let expected_top_n_bids = [None, None, None];
        let expected_top_n_asks = [None, None, None];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        add_qty!(100, 12, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(100),
                qty: 12,
            }),
            None,
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 12,
            }),
            None,
            None,
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        delete_qty!(100, 12, book_sides);

        let expected_top_n_bids = [None, None, None];
        let expected_top_n_asks = [None, None, None];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);
        add_qty!(102, 13, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(102),
                qty: 13,
            }),
            None,
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(102),
                qty: 13,
            }),
            None,
            None,
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);
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
                price: BidPrice(105),
                qty: 15,
            }),
            Some(PriceLevel {
                price: BidPrice(104),
                qty: 14,
            }),
            Some(PriceLevel {
                price: BidPrice(103),
                qty: 13,
            }),
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 10,
            }),
            Some(PriceLevel {
                price: AskPrice(101),
                qty: 11,
            }),
            Some(PriceLevel {
                price: AskPrice(102),
                qty: 12,
            }),
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        delete_qty!(100, 10, book_sides);

        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(101),
                qty: 11,
            }),
            Some(PriceLevel {
                price: AskPrice(102),
                qty: 12,
            }),
            Some(PriceLevel {
                price: AskPrice(103),
                qty: 13,
            }),
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        add_qty!(99, 9, book_sides);

        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(99),
                qty: 9,
            }),
            Some(PriceLevel {
                price: AskPrice(101),
                qty: 11,
            }),
            Some(PriceLevel {
                price: AskPrice(102),
                qty: 12,
            }),
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        delete_qty!(105, 15, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(104),
                qty: 14,
            }),
            Some(PriceLevel {
                price: BidPrice(103),
                qty: 13,
            }),
            Some(PriceLevel {
                price: BidPrice(102),
                qty: 12,
            }),
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        add_qty!(106, 16, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(106),
                qty: 16,
            }),
            Some(PriceLevel {
                price: BidPrice(104),
                qty: 14,
            }),
            Some(PriceLevel {
                price: BidPrice(103),
                qty: 13,
            }),
        ];

        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);
    }

    #[test]
    fn test_set_level() {
        let mut book_sides = create_books();

        // Test setting initial levels
        set_level!(100, 10, book_sides);
        set_level!(105, 15, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(105),
                qty: 15,
            }),
            Some(PriceLevel {
                price: BidPrice(100),
                qty: 10,
            }),
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 10,
            }),
            Some(PriceLevel {
                price: AskPrice(105),
                qty: 15,
            }),
            None,
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        // Test updating existing levels and adding new ones
        set_level!(101, 20, book_sides);
        set_level!(100, 5, book_sides); // Updating existing level
        set_level!(104, 25, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(105),
                qty: 15,
            }),
            Some(PriceLevel {
                price: BidPrice(104),
                qty: 25,
            }),
            Some(PriceLevel {
                price: BidPrice(101),
                qty: 20,
            }),
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(100),
                qty: 5,
            }),
            Some(PriceLevel {
                price: AskPrice(101),
                qty: 20,
            }),
            Some(PriceLevel {
                price: AskPrice(104),
                qty: 25,
            }),
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        // Test removing a level by setting quantity to zero
        set_level!(100, 0, book_sides);
        set_level!(105, 0, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(104),
                qty: 25,
            }),
            Some(PriceLevel {
                price: BidPrice(101),
                qty: 20,
            }),
            None,
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(101),
                qty: 20,
            }),
            Some(PriceLevel {
                price: AskPrice(104),
                qty: 25,
            }),
            None,
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        // Test setting levels beyond top N
        set_level!(102, 30, book_sides);
        set_level!(103, 40, book_sides);
        set_level!(99, 5, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(104),
                qty: 25,
            }),
            Some(PriceLevel {
                price: BidPrice(103),
                qty: 40,
            }),
            Some(PriceLevel {
                price: BidPrice(102),
                qty: 30,
            }),
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(99),
                qty: 5,
            }),
            Some(PriceLevel {
                price: AskPrice(101),
                qty: 20,
            }),
            Some(PriceLevel {
                price: AskPrice(102),
                qty: 30,
            }),
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);

        // Test updating a level to become the new best
        set_level!(98, 35, book_sides);

        let expected_top_n_bids = [
            Some(PriceLevel {
                price: BidPrice(104),
                qty: 25,
            }),
            Some(PriceLevel {
                price: BidPrice(103),
                qty: 40,
            }),
            Some(PriceLevel {
                price: BidPrice(102),
                qty: 30,
            }),
        ];
        let expected_top_n_asks = [
            Some(PriceLevel {
                price: AskPrice(98),
                qty: 35,
            }),
            Some(PriceLevel {
                price: AskPrice(99),
                qty: 5,
            }),
            Some(PriceLevel {
                price: AskPrice(101),
                qty: 20,
            }),
        ];
        assert_top_n_bids!(expected_top_n_bids, book_sides);
        assert_top_n_asks!(expected_top_n_asks, book_sides);
    }
}
