use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::Hash;

use num::Num;

use crate::book_side::{BookSide, DeleteLevelType, FoundLevelType};
use crate::book_side_ops::{BookSideOps, BookSideOpsError};
use crate::price_level::PriceLevel;
use crate::top_n_levels::NLevels;

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
}

impl<
        Price: Debug + Eq + Ord + Copy + Hash,
        Qty: Debug + Ord + Clone + Copy + Num,
        const N: usize,
    > BookSideOps<Price, Qty> for BookSideWithTopNTracking<Price, Qty, N>
{
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
            self.top_n_levels.worst_price.map(|px| px.cmp(&added_price)),
        ) {
            // Ignore bid below worst tracked price or ask above worst tracked price
            (_, true, Some(Ordering::Less)) | (_, false, Some(Ordering::Greater)) => {}
            // Adding qty to existing tracked price
            (FoundLevelType::Existing, _, _) => {
                self.top_n_levels.update_qty(added_price, added_qty);
            }
            // Insert new top_n bid
            (FoundLevelType::New, true, _) => self.top_n_levels.try_insert_sort(PriceLevel {
                price: added_price,
                qty: added_qty,
            }),
            // Insert new top_n ask
            (FoundLevelType::New, false, _) => self.top_n_levels.insert_sort_reversed(PriceLevel {
                price: added_price,
                qty: added_qty,
            }),
        }
        (
            found_level_type,
            PriceLevel {
                price: added_price,
                qty: added_qty,
            },
        )
    }

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
            }
            // Tracked level delete, find next best level and replace
            (DeleteLevelType::Deleted, _, _) => {
                let best_untracked_level = self.get_nth_best_level();
                self.top_n_levels
                    .replace_sort(level.price, best_untracked_level);
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
    fn test_delete_qty() {
        let mut book_side = BookSideWithTopNTracking::<i32, i32, 3>::new(true);
        let (price, qty) = (100, 10);
        book_side.add_qty(price, qty);
        assert_eq!(book_side.best_price(), Some(price));
        assert_eq!(book_side.best_price_qty(), Some(qty));

        book_side.delete_qty(price, qty).unwrap();
        assert_eq!(book_side.book_side.levels.len(), 0);
        assert_eq!(book_side.best_price(), None);
        assert_eq!(book_side.best_price_qty(), None);
    }

    #[test]
    fn test_best_price_after_add_better() {
        let mut book_side = BookSideWithTopNTracking::<i32, i32, 3>::new(true);
        book_side.add_qty(100, 10);
        assert_eq!(book_side.best_price(), Some(100));
        assert_eq!(book_side.best_price_qty(), Some(10));

        book_side.add_qty(101, 20);
        assert_eq!(book_side.best_price(), Some(101));
        assert_eq!(book_side.best_price_qty(), Some(20));

        let mut book_side = BookSideWithTopNTracking::<i32, i32, 3>::new(false);
        book_side.add_qty(101, 20);
        assert_eq!(book_side.best_price(), Some(101));
        assert_eq!(book_side.best_price_qty(), Some(20));

        book_side.add_qty(100, 10);
        assert_eq!(book_side.best_price(), Some(100));
        assert_eq!(book_side.best_price_qty(), Some(10));
    }

    #[test]
    fn test_best_price_modify_quantity() {
        for is_bid in [true, false] {
            let mut book_side = BookSideWithTopNTracking::<i32, i32, 3>::new(is_bid);
            book_side.add_qty(100, 10);
            assert_eq!(book_side.best_price(), Some(100));
            assert_eq!(book_side.best_price_qty(), Some(10));

            book_side.add_qty(100, 20);
            assert_eq!(book_side.best_price(), Some(100));
            assert_eq!(book_side.best_price_qty(), Some(30));

            book_side.delete_qty(100, 15).unwrap();
            assert_eq!(book_side.best_price(), Some(100));
            assert_eq!(book_side.best_price_qty(), Some(15));

            book_side.delete_qty(100, 15).unwrap();
            assert_eq!(book_side.best_price(), None);
            assert_eq!(book_side.best_price_qty(), None);
        }
    }

    #[test]
    fn test_modify_price() {
        let mut book_side = BookSideWithTopNTracking::<i32, i32, 3>::new(true);
        book_side.add_qty(100, 10);
        assert_eq!(book_side.best_price(), Some(100));
        assert_eq!(book_side.best_price_qty(), Some(10));

        book_side.delete_qty(100, 10).unwrap();
        book_side.add_qty(101, 20);
        assert_eq!(book_side.best_price(), Some(101));
        assert_eq!(book_side.best_price_qty(), Some(20));

        book_side.delete_qty(101, 20).unwrap();
        book_side.add_qty(100, 15);
        assert_eq!(book_side.best_price(), Some(100));
        assert_eq!(book_side.best_price_qty(), Some(15));
    }
}
