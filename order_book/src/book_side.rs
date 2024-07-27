use std::fmt::Debug;
use std::hash::Hash;

use hashbrown::HashMap;
use itertools::Itertools;
use num::traits::Num;
use tracing::{debug, instrument};

use crate::book_side_ops::{BookSideOps, BookSideOpsError, DeleteError, LevelError};

use super::price_level::PriceLevel;

#[derive(Clone, Copy)]
pub enum FoundLevelType {
    New,
    Existing,
}

#[derive(Clone, Copy)]
pub enum DeleteLevelType {
    Deleted,
    QuantityDecreased,
}

pub struct BookSide<Price, Qty> {
    pub is_bid: bool,
    pub levels: HashMap<Price, PriceLevel<Price, Qty>>,
}

impl<Price: Debug, Qty: Debug> Debug for BookSide<Price, Qty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.is_bid {
            true => write!(f, "BidBookSide {{ levels: {:?} }}", self.levels),
            false => write!(f, "AskBookSide {{ levels: {:?} }}", self.levels),
        }
    }
}
impl<Price: Debug + Copy + Eq + Ord + Hash, Qty: Debug + Copy + PartialEq + Ord + Num>
    BookSide<Price, Qty>
{
    #[must_use]
    pub fn new(is_bid: bool) -> Self {
        BookSide {
            is_bid,
            levels: HashMap::new(),
        }
    }

    #[inline]
    pub fn get_level(&self, price: Price) -> Option<&PriceLevel<Price, Qty>> {
        self.levels.get(&price)
    }

    #[inline]
    pub fn get_nth_best_level(&self, n: usize) -> Option<PriceLevel<Price, Qty>> {
        // TODO-optimisation: Consider replacing self.levels HashMap with a BTreeMap.
        // This function will be costly when called too often & when there are many
        // levels.
        let mut sorted = self
            .levels
            .iter()
            .sorted_unstable_by_key(|(price, _)| *price)
            .map(|(_, level)| *level);
        debug!("Getting {:?}'th best level", n);
        if self.is_bid {
            sorted.nth_back(n)
        } else {
            sorted.nth(n)
        }
    }

    #[instrument]
    #[inline]
    pub fn find_or_create_level(
        &mut self,
        price: Price,
    ) -> (FoundLevelType, &mut PriceLevel<Price, Qty>) {
        match self.levels.entry(price) {
            hashbrown::hash_map::Entry::Occupied(o) => (FoundLevelType::Existing, o.into_mut()),
            hashbrown::hash_map::Entry::Vacant(v) => {
                debug!("Created a new price level");
                (FoundLevelType::New, v.insert(PriceLevel::new(price)))
            }
        }
    }
}

impl<Price: Debug + Copy + Eq + Ord + Hash, Qty: Debug + Copy + PartialEq + Ord + Num>
    BookSideOps<Price, Qty> for BookSide<Price, Qty>
{
    #[instrument]
    #[inline]
    fn add_qty(&mut self, price: Price, qty: Qty) -> (FoundLevelType, PriceLevel<Price, Qty>) {
        debug!("Adding quantity to book_side");
        let (found_level_type, level) = self.find_or_create_level(price);
        level.add_qty(qty);
        (found_level_type, *level)
    }

    #[instrument]
    #[inline]
    fn delete_qty(
        &mut self,
        price: Price,
        qty: Qty,
    ) -> Result<(DeleteLevelType, PriceLevel<Price, Qty>), BookSideOpsError> {
        debug!("Called delete_qty");
        let level =
            self.levels
                .get_mut(&price)
                .ok_or(BookSideOpsError::from(DeleteError::from(
                    LevelError::LevelNotFound,
                )))?;
        match level.qty.cmp(&qty) {
            std::cmp::Ordering::Less => Err(DeleteError::QtyExceedsAvailable.into()),
            std::cmp::Ordering::Equal => {
                let deleted_level = self.levels.remove(&price).unwrap();
                Ok((DeleteLevelType::Deleted, deleted_level))
            }
            std::cmp::Ordering::Greater => {
                level.delete_qty(qty);
                Ok((DeleteLevelType::QuantityDecreased, *level))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_book_side_with_orders(is_bid: bool) -> BookSide<u32, u32> {
        let mut book_side = BookSide::new(is_bid);
        book_side.add_qty(1, 100);
        book_side.add_qty(2, 100);
        book_side.add_qty(3, 101);
        book_side.add_qty(4, 98);
        book_side
    }
    #[test]
    fn test_new() {
        let book_side: BookSide<u32, u32> = BookSide::new(true);
        assert!(book_side.is_bid);
        assert_eq!(book_side.levels.len(), 0);

        let book_side: BookSide<u32, u32> = BookSide::new(false);
        assert!(!book_side.is_bid);
        assert_eq!(book_side.levels.len(), 0);
    }

    #[test]
    fn test_add_qty_to_empty_book() {
        for is_bid in [false, true] {
            let qty = 5;
            let price = 100;
            let mut book_side = BookSide::new(is_bid);
            book_side.add_qty(price, qty);
            assert_qty_added(&book_side, price, qty, 0, 0);
        }
    }

    #[test]
    fn test_add_qty() {
        struct TestCase {
            price: u32,
            qty: u32,
        }

        for is_bid in [false, true] {
            let test_cases = vec![
                TestCase {
                    price: 100,
                    qty: 10,
                },
                TestCase {
                    price: 100,
                    qty: 20,
                },
                TestCase {
                    price: 101,
                    qty: 30,
                },
                TestCase { price: 98, qty: 40 },
            ];
            for TestCase { price, qty } in test_cases {
                let mut book_side = create_book_side_with_orders(is_bid);
                let num_levels_before = book_side.levels.len();
                let qty_before = book_side.levels.get(&price).map_or(0, |l| l.qty);
                book_side.add_qty(price, qty);
                assert_qty_added(&book_side, price, qty, qty_before, num_levels_before);
            }
        }
    }

    fn assert_qty_added(
        book_side: &BookSide<u32, u32>,
        price: u32,
        qty: u32,
        qty_before: u32,
        num_levels_before: usize,
    ) {
        let new_level_created = qty_before == 0;
        assert_eq!(
            book_side.levels.len(),
            num_levels_before + new_level_created as usize
        );
        let level = book_side.levels.get(&price).expect("Level not found");
        assert_eq!(level.price, price);
        assert_eq!(level.qty, qty_before + qty);
    }

    #[test]
    fn test_delete_qty() {
        let mut book_side = BookSide::new(true);
        let (price, qty) = (100, 10);
        book_side.add_qty(price, qty);
        book_side.delete_qty(price, qty).unwrap();
        assert_eq!(book_side.levels.len(), 0);
    }

    #[test]
    fn test_get_nth_best_level_bid() {
        let mut book_side = create_book_side_with_orders(true);

        assert_eq!(
            book_side.get_nth_best_level(0),
            Some(PriceLevel { price: 4, qty: 98 })
        );

        assert_eq!(
            book_side.get_nth_best_level(1),
            Some(PriceLevel { price: 3, qty: 101 })
        );

        assert_eq!(
            book_side.get_nth_best_level(2),
            Some(PriceLevel { price: 2, qty: 100 })
        );

        assert_eq!(
            book_side.get_nth_best_level(3),
            Some(PriceLevel { price: 1, qty: 100 })
        );

        // Non-existent level
        assert_eq!(book_side.get_nth_best_level(4), None);

        book_side.delete_qty(3, 101).unwrap();
        assert_eq!(
            book_side.get_nth_best_level(0),
            Some(PriceLevel { price: 4, qty: 98 })
        );
        assert_eq!(
            book_side.get_nth_best_level(1),
            Some(PriceLevel { price: 2, qty: 100 })
        );
        assert_eq!(
            book_side.get_nth_best_level(2),
            Some(PriceLevel { price: 1, qty: 100 })
        );
        assert_eq!(book_side.get_nth_best_level(3), None);

        book_side.delete_qty(1, 100).unwrap();
        assert_eq!(
            book_side.get_nth_best_level(0),
            Some(PriceLevel { price: 4, qty: 98 })
        );
        assert_eq!(
            book_side.get_nth_best_level(1),
            Some(PriceLevel { price: 2, qty: 100 })
        );
        assert_eq!(book_side.get_nth_best_level(2), None);

        book_side.delete_qty(4, 98).unwrap();
        assert_eq!(
            book_side.get_nth_best_level(0),
            Some(PriceLevel { price: 2, qty: 100 })
        );
        assert_eq!(book_side.get_nth_best_level(1), None);

    }

    #[test]
    fn test_get_nth_best_level_ask() {
        let mut book_side = create_book_side_with_orders(false);

        assert_eq!(
            book_side.get_nth_best_level(0),
            Some(PriceLevel { price: 1, qty: 100 })
        );

        assert_eq!(
            book_side.get_nth_best_level(1),
            Some(PriceLevel { price: 2, qty: 100 })
        );

        assert_eq!(
            book_side.get_nth_best_level(2),
            Some(PriceLevel { price: 3, qty: 101 })
        );

        assert_eq!(
            book_side.get_nth_best_level(3),
            Some(PriceLevel { price: 4, qty: 98 })
        );

        // Non-existent level
        assert_eq!(book_side.get_nth_best_level(4), None);

        book_side.delete_qty(1, 100).unwrap();
        assert_eq!(
            book_side.get_nth_best_level(0),
            Some(PriceLevel { price: 2, qty: 100 })
        );
        assert_eq!(
            book_side.get_nth_best_level(1),
            Some(PriceLevel { price: 3, qty: 101 })
        );
        assert_eq!(
            book_side.get_nth_best_level(2),
            Some(PriceLevel { price: 4, qty: 98 })
        );
        assert_eq!(book_side.get_nth_best_level(3), None);

        book_side.delete_qty(4, 98).unwrap();
        assert_eq!(
            book_side.get_nth_best_level(0),
            Some(PriceLevel { price: 2, qty: 100 })
        );
        assert_eq!(
            book_side.get_nth_best_level(1),
            Some(PriceLevel { price: 3, qty: 101 })
        );
        assert_eq!(book_side.get_nth_best_level(2), None);
    }
}
