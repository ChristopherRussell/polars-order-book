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

// trait BookSideOps<Price, Qty> {
//     fn get_level(&self, price: Price) -> Option<&PriceLevel<Price, Qty>>;
//     fn find_or_create_level(
//         &mut self,
//         price: Price,
//     ) -> (FoundLevelType, &mut PriceLevel<Price, Qty>);
//     fn add_qty(&mut self, price: Price, qty: Qty);
//     fn delete_qty(&mut self, price: Price, qty: Qty) -> Result<(), DeleteError>;
// }

#[derive(Debug)]
pub struct BookSide<Price, Qty> {
    is_bid: bool,
    levels: HashMap<Price, PriceLevel<Price, Qty>>,
    pub best_price: Option<Price>,
    pub best_price_qty: Option<Qty>,
}

impl<Price: Debug + Copy + Eq + Ord + Hash, Qty: Debug + Copy + PartialEq + Ord + Num>
BookSide<Price, Qty>
{
    #[must_use]
    pub fn new(is_bid: bool) -> Self {
        BookSide {
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
            hashbrown::hash_map::Entry::Vacant(v) => (
                FoundLevelType::New,
                v.insert(PriceLevel::new(price)),
            ),
        }
    }

    #[inline]
    fn update_best_price_after_add(&mut self, added_price: Price, added_qty: Qty) {
        match (self.is_bid, self.best_price) {
            (true, Some(best_price)) => {
                if added_price < best_price {
                    return;
                }
            }
            (false, Some(best_price)) => {
                if added_price > best_price {
                    return;
                }
            }
            _ => {}
        }
        self.best_price = Some(added_price);
        self.best_price_qty = Some(added_qty);
    }

    #[inline]
    fn update_best_price_after_delete(&mut self, deleted_price: Price) {
        if self.best_price == Some(deleted_price) {
            (self.best_price, self.best_price_qty) = self
                .get_best_price_level()
                .map_or((None, None), |l| (Some(l.price), Some(l.qty)));
        }
    }

    #[inline]
    pub fn add_qty(&mut self, price: Price, qty: Qty) {
        let (found_level_type, level) = self.find_or_create_level(price);
        level.add_qty(qty);
        if let FoundLevelType::New = found_level_type {
            self.update_best_price_after_add(price, qty);
        }
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
                self.update_best_price_after_delete(price);
            }
            std::cmp::Ordering::Greater => {
                level.delete_qty(qty);
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

    fn create_book_side_with_orders() -> BookSide<u32, u32> {
        let mut book_side = BookSide::new(true);
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
        let qty = 5;
        let price = 100;
        let mut book_side = BookSide::new(true);
        assert_eq!(book_side.best_price, None);
        assert_eq!(book_side.best_price_qty, None);
        book_side.add_qty(price, qty);
        assert_qty_added(&book_side, price, qty, 0, 0);

        let mut expected_level = PriceLevel::new(price);
        expected_level.add_qty(qty);
        let level = book_side.levels.get(&price).unwrap();
        assert_eq!(level, &expected_level);
        assert_eq!(book_side.best_price, Some(price));
        assert_eq!(book_side.best_price_qty, Some(qty));
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
        assert_eq!(book_side.best_price, Some(price));
        assert_eq!(book_side.best_price_qty, Some(qty));

        book_side.delete_qty(price, qty).unwrap();
        assert_eq!(book_side.levels.len(), 0);
        assert_eq!(book_side.best_price, None);
        assert_eq!(book_side.best_price_qty, None);
    }
}
