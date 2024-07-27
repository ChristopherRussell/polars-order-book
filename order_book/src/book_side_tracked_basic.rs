use std::fmt::Debug;
use std::hash::Hash;

use hashbrown::HashMap;
use num::traits::Num;
use thiserror::Error;

use super::price_level::PriceLevel;

pub enum FoundLevelType {
    New,
    Existing,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum LevelError {
    #[error("Level not found")]
    LevelNotFound,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum DeleteError {
    #[error(transparent)]
    LevelError(#[from] LevelError),
    #[error("Qty exceeds available")]
    QtyExceedsAvailable,
}

#[derive(Debug)]
pub struct BookSideWithBasicTracking<Price, Qty> {
    is_bid: bool,
    levels: HashMap<Price, PriceLevel<Price, Qty>>,
    pub best_price: Option<Price>,
    pub best_price_qty: Option<Qty>,
}

impl<Price: Debug + Copy + Eq + Ord + Hash, Qty: Debug + Copy + PartialEq + Ord + Num>
    BookSideWithBasicTracking<Price, Qty>
{
    #[must_use]
    pub fn new(is_bid: bool) -> Self {
        BookSideWithBasicTracking {
            is_bid,
            levels: HashMap::new(),
            best_price: None,
            best_price_qty: None,
        }
    }

    #[inline]
    pub fn get_level(&self, price: Price) -> Option<&PriceLevel<Price, Qty>> {
        self.levels.get(&price)
    }

    #[inline]
    pub fn find_or_create_level(
        &mut self,
        price: Price,
    ) -> (FoundLevelType, &mut PriceLevel<Price, Qty>) {
        match self.levels.entry(price) {
            hashbrown::hash_map::Entry::Occupied(o) => (FoundLevelType::Existing, o.into_mut()),
            hashbrown::hash_map::Entry::Vacant(v) => {
                (FoundLevelType::New, v.insert(PriceLevel::new(price)))
            }
        }
    }

    #[inline]
    fn update_best_price_after_add(
        &mut self,
        found_level_type: FoundLevelType,
        added_price: Price,
        added_qty: Qty,
    ) {
        match (
            found_level_type,
            self.is_bid,
            self.best_price.map(|px| px.cmp(&added_price)),
        ) {
            // Adding qty to existing best price
            (FoundLevelType::Existing, _, Some(std::cmp::Ordering::Equal)) => {
                self.best_price_qty = self.best_price_qty.map(|qty| qty + added_qty);
            }
            // New price is better than current best price
            (FoundLevelType::New, _, None)
            | (FoundLevelType::New, true, Some(std::cmp::Ordering::Less))
            | (FoundLevelType::New, false, Some(std::cmp::Ordering::Greater)) => {
                self.best_price = Some(added_price);
                self.best_price_qty = Some(added_qty);
            }
            (FoundLevelType::New, _, Some(std::cmp::Ordering::Equal)) => panic!(
                "update_best_price_after_add: New level has same price as current best price"
            ),
            (FoundLevelType::Existing, _, None) => {
                panic!(
                    "update_best_price_after_add: If there is an existing level then best price should not be None"
                )
            }
            _ => {}
        }
    }

    #[inline]
    fn update_best_price_after_level_delete(&mut self, deleted_price: Price) {
        if self.best_price == Some(deleted_price) {
            (self.best_price, self.best_price_qty) = self
                .get_best_price_level()
                .map_or((None, None), |l| (Some(l.price), Some(l.qty)));
        }
    }

    #[inline]
    fn update_best_price_after_qty_delete(&mut self, deleted_price: Price, deleted_qty: Qty) {
        if self.best_price == Some(deleted_price) {
            self.best_price_qty = self.best_price_qty.map(|qty| qty - deleted_qty);
        }
    }

    #[inline]
    pub fn add_qty(&mut self, price: Price, qty: Qty) {
        let (found_level_type, level) = self.find_or_create_level(price);
        level.add_qty(qty);
        self.update_best_price_after_add(found_level_type, price, qty);
    }

    #[inline]
    pub fn delete_qty(&mut self, price: Price, qty: Qty) -> Result<(), DeleteError> {
        let level = self
            .levels
            .get_mut(&price)
            .ok_or(LevelError::LevelNotFound)?;
        match level.qty.cmp(&qty) {
            std::cmp::Ordering::Less => return Err(DeleteError::QtyExceedsAvailable),
            std::cmp::Ordering::Equal => {
                self.levels.remove(&price);
                self.update_best_price_after_level_delete(price);
            }
            std::cmp::Ordering::Greater => {
                level.delete_qty(qty);
                self.update_best_price_after_qty_delete(price, qty);
            }
        }
        Ok(())
    }

    #[inline]
    pub fn get_best_price_level(&self) -> Option<&PriceLevel<Price, Qty>> {
        if self.is_bid {
            self.levels.values().max_by_key(|l| l.price)
        } else {
            self.levels.values().min_by_key(|l| l.price)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_book_side_with_orders() -> BookSideWithBasicTracking<u32, u32> {
        let mut book_side = BookSideWithBasicTracking::new(true);
        book_side.add_qty(1, 100);
        book_side.add_qty(2, 100);
        book_side.add_qty(3, 101);
        book_side.add_qty(4, 98);
        book_side
    }

    #[test]
    fn test_new() {
        let book_side: BookSideWithBasicTracking<u32, u32> = BookSideWithBasicTracking::new(true);
        assert!(book_side.is_bid);
        assert_eq!(book_side.levels.len(), 0);

        let book_side: BookSideWithBasicTracking<u32, u32> = BookSideWithBasicTracking::new(false);
        assert!(!book_side.is_bid);
        assert_eq!(book_side.levels.len(), 0);
    }

    #[test]
    fn test_add_qty_to_empty_book() {
        for is_bid in vec![false, true] {
            let qty = 5;
            let price = 100;
            let mut book_side = BookSideWithBasicTracking::new(is_bid);
            assert_eq!(book_side.best_price, None);
            assert_eq!(book_side.best_price_qty, None);
            book_side.add_qty(price, qty);
            assert_qty_added(&book_side, price, qty, 0, 0);
            assert_eq!(book_side.best_price, Some(price));
            assert_eq!(book_side.best_price_qty, Some(qty));
        }
    }

    #[test]
    fn test_add_qty() {
        struct TestCase {
            price: u32,
            qty: u32,
        }

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
            let mut book_side = create_book_side_with_orders();
            let num_levels_before = book_side.levels.len();
            let qty_before = book_side.levels.get(&price).map_or(0, |l| l.qty);
            book_side.add_qty(price, qty);
            assert_qty_added(&book_side, price, qty, qty_before, num_levels_before);
        }
    }

    fn assert_qty_added(
        book_side: &BookSideWithBasicTracking<u32, u32>,
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
        let mut book_side = BookSideWithBasicTracking::new(true);
        let (price, qty) = (100, 10);
        book_side.add_qty(price, qty);
        assert_eq!(book_side.best_price, Some(price));
        assert_eq!(book_side.best_price_qty, Some(qty));

        book_side.delete_qty(price, qty).unwrap();
        assert_eq!(book_side.levels.len(), 0);
        assert_eq!(book_side.best_price, None);
        assert_eq!(book_side.best_price_qty, None);
    }

    #[test]
    fn test_best_price_after_add_better() {
        let mut book_side = BookSideWithBasicTracking::new(true);
        book_side.add_qty(100, 10);
        assert_eq!(book_side.best_price, Some(100));
        assert_eq!(book_side.best_price_qty, Some(10));

        book_side.add_qty(101, 20);
        assert_eq!(book_side.best_price, Some(101));
        assert_eq!(book_side.best_price_qty, Some(20));

        let mut book_side = BookSideWithBasicTracking::new(false);
        book_side.add_qty(101, 20);
        assert_eq!(book_side.best_price, Some(101));
        assert_eq!(book_side.best_price_qty, Some(20));

        book_side.add_qty(100, 10);
        assert_eq!(book_side.best_price, Some(100));
        assert_eq!(book_side.best_price_qty, Some(10));
    }

    #[test]
    fn test_best_price_modify_quantity() {
        for is_bid in vec![true, false] {
            let mut book_side = BookSideWithBasicTracking::new(is_bid);
            book_side.add_qty(100, 10);
            assert_eq!(book_side.best_price, Some(100));
            assert_eq!(book_side.best_price_qty, Some(10));

            book_side.add_qty(100, 20);
            assert_eq!(book_side.best_price, Some(100));
            assert_eq!(book_side.best_price_qty, Some(30));

            book_side.delete_qty(100, 15).unwrap();
            assert_eq!(book_side.best_price, Some(100));
            assert_eq!(book_side.best_price_qty, Some(15));

            book_side.delete_qty(100, 15).unwrap();
            assert_eq!(book_side.best_price, None);
            assert_eq!(book_side.best_price_qty, None);
        }
    }

    #[test]
    fn test_modify_price() {
        let mut book_side = BookSideWithBasicTracking::new(true);
        book_side.add_qty(100, 10);
        assert_eq!(book_side.best_price, Some(100));
        assert_eq!(book_side.best_price_qty, Some(10));

        book_side.delete_qty(100, 10).unwrap();
        book_side.add_qty(101, 20);
        assert_eq!(book_side.best_price, Some(101));
        assert_eq!(book_side.best_price_qty, Some(20));

        book_side.delete_qty(101, 20).unwrap();
        book_side.add_qty(100, 15);
        assert_eq!(book_side.best_price, Some(100));
        assert_eq!(book_side.best_price_qty, Some(15));
    }
}
