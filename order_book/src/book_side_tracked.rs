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
        self.book_side.get_nth_best_level(N)
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
    use super::*;

    #[test]
    fn test_add_more_levels_than_tracked() {
        let mut book_side_bid1 = BookSideWithTopNTracking::<i32, i32, 1>::new(true);
        let mut book_side_bid2 = BookSideWithTopNTracking::<i32, i32, 2>::new(true);
        let mut book_side_ask1 = BookSideWithTopNTracking::<i32, i32, 1>::new(false);
        let mut book_side_ask2 = BookSideWithTopNTracking::<i32, i32, 2>::new(false);
        let prices = [400, 100, 200, 300, 400, 100];
        let qtys = [19, 6, 20, 30, 21, 4];
        for (price, qty) in prices.iter().zip(qtys.iter()) {
            for book_side in [&mut book_side_bid1, &mut book_side_ask1] {
                book_side.add_qty(*price, *qty);
            }
            for book_side in [&mut book_side_bid2, &mut book_side_ask2] {
                book_side.add_qty(*price, *qty);
            }
        }
        assert_eq!(book_side_bid1.best_price(), Some(400));
        assert_eq!(book_side_bid1.best_price_qty(), Some(40));
        assert_eq!(
            book_side_bid1.top_n(),
            &[Some(PriceLevel {
                price: 400,
                qty: 40
            })]
        );
        assert_eq!(book_side_ask1.best_price(), Some(100));
        assert_eq!(book_side_ask1.best_price_qty(), Some(10));
        assert_eq!(
            book_side_ask1.top_n(),
            &[Some(PriceLevel {
                price: 100,
                qty: 10
            })]
        );
        assert_eq!(book_side_bid2.best_price(), Some(400));
        assert_eq!(book_side_bid2.best_price_qty(), Some(40));
        assert_eq!(
            book_side_bid2.top_n(),
            &[
                Some(PriceLevel {
                    price: 400,
                    qty: 40
                }),
                Some(PriceLevel {
                    price: 300,
                    qty: 30
                })
            ]
        );
        assert_eq!(book_side_ask2.best_price(), Some(100));
        assert_eq!(book_side_ask2.best_price_qty(), Some(10));
        assert_eq!(
            book_side_ask2.top_n(),
            &[
                Some(PriceLevel {
                    price: 100,
                    qty: 10
                }),
                Some(PriceLevel {
                    price: 200,
                    qty: 20
                })
            ]
        );
    }

    #[test]
    fn test_delete_qty() {
        let mut book_side = BookSideWithTopNTracking::<i32, i32, 3>::new(true);
        let (price, qty) = (100, 10);
        book_side.add_qty(price, qty);
        assert_eq!(book_side.best_price(), Some(price));
        assert_eq!(book_side.best_price_qty(), Some(qty));
        assert_eq!(
            book_side.top_n(),
            &[Some(PriceLevel { price, qty }), None, None]
        );

        book_side.delete_qty(price, qty).unwrap();
        assert_eq!(book_side.book_side.levels.len(), 0);
        assert_eq!(book_side.best_price(), None);
        assert_eq!(book_side.best_price_qty(), None);
        assert_eq!(book_side.top_n(), &[None, None, None]);
    }

    #[test]
    fn test_best_price_after_add_better() {
        let mut book_side = BookSideWithTopNTracking::<i32, i32, 3>::new(true);
        book_side.add_qty(100, 10);
        assert_eq!(book_side.best_price(), Some(100));
        assert_eq!(book_side.best_price_qty(), Some(10));
        assert_eq!(
            book_side.top_n(),
            &[
                Some(PriceLevel {
                    price: 100,
                    qty: 10
                }),
                None,
                None
            ]
        );

        book_side.add_qty(101, 20);
        assert_eq!(book_side.best_price(), Some(101));
        assert_eq!(book_side.best_price_qty(), Some(20));
        assert_eq!(
            book_side.top_n(),
            &[
                Some(PriceLevel {
                    price: 101,
                    qty: 20
                }),
                Some(PriceLevel {
                    price: 100,
                    qty: 10
                }),
                None
            ]
        );

        let mut book_side = BookSideWithTopNTracking::<i32, i32, 3>::new(false);
        book_side.add_qty(101, 20);
        assert_eq!(book_side.best_price(), Some(101));
        assert_eq!(book_side.best_price_qty(), Some(20));
        assert_eq!(
            book_side.top_n(),
            &[
                Some(PriceLevel {
                    price: 101,
                    qty: 20
                }),
                None,
                None
            ]
        );

        book_side.add_qty(100, 10);
        assert_eq!(book_side.best_price(), Some(100));
        assert_eq!(book_side.best_price_qty(), Some(10));
        assert_eq!(
            book_side.top_n(),
            &[
                Some(PriceLevel {
                    price: 100,
                    qty: 10
                }),
                Some(PriceLevel {
                    price: 101,
                    qty: 20
                }),
                None
            ]
        );
    }

    #[test]
    fn test_best_price_modify_quantity() {
        for is_bid in [true, false] {
            let mut book_side = BookSideWithTopNTracking::<i32, i32, 3>::new(is_bid);
            book_side.add_qty(100, 10);
            assert_eq!(book_side.best_price(), Some(100));
            assert_eq!(book_side.best_price_qty(), Some(10));
            assert_eq!(
                book_side.top_n(),
                &[
                    Some(PriceLevel {
                        price: 100,
                        qty: 10
                    }),
                    None,
                    None
                ]
            );

            book_side.add_qty(100, 20);
            assert_eq!(book_side.best_price(), Some(100));
            assert_eq!(book_side.best_price_qty(), Some(30));
            assert_eq!(
                book_side.top_n(),
                &[
                    Some(PriceLevel {
                        price: 100,
                        qty: 30
                    }),
                    None,
                    None
                ]
            );

            book_side.delete_qty(100, 15).unwrap();
            assert_eq!(book_side.best_price(), Some(100));
            assert_eq!(book_side.best_price_qty(), Some(15));
            assert_eq!(
                book_side.top_n(),
                &[
                    Some(PriceLevel {
                        price: 100,
                        qty: 15
                    }),
                    None,
                    None
                ]
            );

            book_side.delete_qty(100, 15).unwrap();
            assert_eq!(book_side.best_price(), None);
            assert_eq!(book_side.best_price_qty(), None);
            assert_eq!(book_side.top_n(), &[None, None, None]);
        }
    }

    #[test]
    fn test_modify_price() {
        let mut book_side1 = BookSideWithTopNTracking::<i32, i32, 1>::new(true);
        let mut book_side2 = BookSideWithTopNTracking::<i32, i32, 2>::new(true);
        let mut book_side3 = BookSideWithTopNTracking::<i32, i32, 3>::new(true);
        book_side1.add_qty(100, 10);
        book_side2.add_qty(100, 10);
        book_side3.add_qty(100, 10);

        assert_eq!(book_side1.best_price(), Some(100));
        assert_eq!(book_side2.best_price(), Some(100));
        assert_eq!(book_side3.best_price(), Some(100));

        assert_eq!(book_side1.best_price_qty(), Some(10));
        assert_eq!(book_side2.best_price_qty(), Some(10));
        assert_eq!(book_side3.best_price_qty(), Some(10));

        let top_n = [
            Some(PriceLevel {
                price: 100,
                qty: 10,
            }),
            None,
            None,
        ];
        assert_eq!(book_side1.top_n(), &top_n[..1]);
        assert_eq!(book_side2.top_n(), &top_n[..2]);
        assert_eq!(book_side3.top_n(), &top_n);

        book_side1.delete_qty(100, 10).unwrap();
        book_side1.add_qty(101, 20);
        book_side2.delete_qty(100, 10).unwrap();
        book_side2.add_qty(101, 20);
        book_side3.delete_qty(100, 10).unwrap();
        book_side3.add_qty(101, 20);

        assert_eq!(book_side1.best_price(), Some(101));
        assert_eq!(book_side2.best_price(), Some(101));
        assert_eq!(book_side3.best_price(), Some(101));

        assert_eq!(book_side1.best_price_qty(), Some(20));
        assert_eq!(book_side2.best_price_qty(), Some(20));
        assert_eq!(book_side3.best_price_qty(), Some(20));

        let top_n = [
            Some(PriceLevel {
                price: 101,
                qty: 20,
            }),
            None,
            None,
        ];

        assert_eq!(book_side1.top_n(), &top_n[..1]);
        assert_eq!(book_side2.top_n(), &top_n[..2]);
        assert_eq!(book_side3.top_n(), &top_n);

        book_side1.delete_qty(101, 20).unwrap();
        book_side1.add_qty(100, 15);
        book_side2.delete_qty(101, 20).unwrap();
        book_side2.add_qty(100, 15);
        book_side3.delete_qty(101, 20).unwrap();
        book_side3.add_qty(100, 15);

        assert_eq!(book_side1.best_price(), Some(100));
        assert_eq!(book_side2.best_price(), Some(100));
        assert_eq!(book_side3.best_price(), Some(100));

        assert_eq!(book_side1.best_price_qty(), Some(15));
        assert_eq!(book_side2.best_price_qty(), Some(15));
        assert_eq!(book_side3.best_price_qty(), Some(15));

        let top_n = [
            Some(PriceLevel {
                price: 100,
                qty: 15,
            }),
            None,
            None,
        ];
        assert_eq!(book_side1.top_n(), &top_n[..1]);
        assert_eq!(book_side2.top_n(), &top_n[..2]);
        assert_eq!(book_side3.top_n(), &top_n);
    }

    #[test]
    fn test_book_side_with_cyclic_modify_price() {
        let mut bid_side_1 = BookSideWithTopNTracking::<i32, i32, 1>::new(true);
        let mut bid_side_2 = BookSideWithTopNTracking::<i32, i32, 2>::new(true);
        let mut ask_side_1 = BookSideWithTopNTracking::<i32, i32, 1>::new(false);
        let mut ask_side_2 = BookSideWithTopNTracking::<i32, i32, 2>::new(false);

        bid_side_1.add_qty(100, 10);
        bid_side_2.add_qty(100, 10);
        ask_side_1.add_qty(100, 10);
        ask_side_2.add_qty(100, 10);

        bid_side_1.delete_qty(100, 10).unwrap();
        bid_side_2.delete_qty(100, 10).unwrap();
        ask_side_1.delete_qty(100, 10).unwrap();
        ask_side_2.delete_qty(100, 10).unwrap();

        bid_side_1.add_qty(101, 11);
        bid_side_2.add_qty(101, 11);
        ask_side_1.add_qty(101, 11);
        ask_side_2.add_qty(101, 11);

        let top_n = [
            Some(PriceLevel {
                price: 101,
                qty: 11,
            }),
            None,
        ];
        assert_eq!(bid_side_1.top_n(), &top_n[..1]);
        assert_eq!(bid_side_2.top_n(), &top_n);
        assert_eq!(ask_side_1.top_n(), &top_n[..1]);
        assert_eq!(ask_side_2.top_n(), &top_n);

        bid_side_1.delete_qty(101, 11).unwrap();
        bid_side_2.delete_qty(101, 11).unwrap();
        ask_side_1.delete_qty(101, 11).unwrap();
        ask_side_2.delete_qty(101, 11).unwrap();

        let top_n = [None, None];
        assert_eq!(bid_side_1.top_n(), &top_n[..1]);
        assert_eq!(bid_side_2.top_n(), &top_n);
        assert_eq!(ask_side_1.top_n(), &top_n[..1]);
        assert_eq!(ask_side_2.top_n(), &top_n);

        bid_side_1.add_qty(100, 12);
        bid_side_2.add_qty(100, 12);
        ask_side_1.add_qty(100, 12);
        ask_side_2.add_qty(100, 12);

        let top_n = [
            Some(PriceLevel {
                price: 100,
                qty: 12,
            }),
            None,
        ];
        assert_eq!(bid_side_1.top_n(), &top_n[..1]);
        assert_eq!(bid_side_2.top_n(), &top_n);
        assert_eq!(ask_side_1.top_n(), &top_n[..1]);
        assert_eq!(ask_side_2.top_n(), &top_n);

        bid_side_1.delete_qty(100, 12).unwrap();
        bid_side_2.delete_qty(100, 12).unwrap();
        ask_side_1.delete_qty(100, 12).unwrap();
        ask_side_2.delete_qty(100, 12).unwrap();

        let top_n = [None, None];

        assert_eq!(bid_side_1.top_n(), &top_n[..1]);
        assert_eq!(bid_side_2.top_n(), &top_n);
        assert_eq!(ask_side_1.top_n(), &top_n[..1]);
        assert_eq!(ask_side_2.top_n(), &top_n);

        bid_side_1.add_qty(102, 13);
        bid_side_2.add_qty(102, 13);
        ask_side_1.add_qty(102, 13);
        ask_side_2.add_qty(102, 13);

        let top_n = [
            Some(PriceLevel {
                price: 102,
                qty: 13,
            }),
            None,
        ];

        assert_eq!(bid_side_1.top_n(), &top_n[..1]);
        assert_eq!(bid_side_2.top_n(), &top_n);
        assert_eq!(ask_side_1.top_n(), &top_n[..1]);
        assert_eq!(ask_side_2.top_n(), &top_n);
    }
}
