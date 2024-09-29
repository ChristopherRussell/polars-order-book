use hashbrown::HashMap;
use order_book_core::book_side::{BookSide, DeleteLevelType, FoundLevelType};
use order_book_core::book_side_ops::{
    LevelError, PricePointMutationOps, PricePointMutationOpsError, PricePointSummaryOps,
};
use order_book_core::price_level::{self, QuantityLike};
use std::fmt::Debug;

#[derive(Debug)]
pub struct BookSideWithBasicTracking<Px: price_level::Price, Qty: QuantityLike> {
    pub levels: HashMap<Px, Qty>,
    pub best_price: Option<Px>,
    pub best_price_qty: Option<Qty>,
}

impl<Px: price_level::Price, Qty: QuantityLike> BookSide<Px, Qty>
    for BookSideWithBasicTracking<Px, Qty>
{
    fn levels(&self) -> &HashMap<Px, Qty> {
        &self.levels
    }
    fn levels_mut(&mut self) -> &mut HashMap<Px, Qty> {
        &mut self.levels
    }
}

impl<Px: price_level::Price, Qty: QuantityLike> BookSideWithBasicTracking<Px, Qty> {
    pub fn new() -> Self {
        Self {
            levels: HashMap::new(),
            best_price: None,
            best_price_qty: None,
        }
    }
}

impl<Px: price_level::Price, Qty: QuantityLike> Default for BookSideWithBasicTracking<Px, Qty> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Px: price_level::Price, Qty: QuantityLike> BookSideWithBasicTracking<Px, Qty> {
    #[inline]
    fn update_best_price_after_add(&mut self, added_price: Px, new_qty: Qty) {
        match self.best_price.map(|px| px.cmp(&added_price)) {
            // New price is better than current best price
            None | Some(std::cmp::Ordering::Less) => {
                self.best_price = Some(added_price);
                self.best_price_qty = Some(new_qty);
            }
            // Adding qty to existing best price
            Some(std::cmp::Ordering::Equal) => {
                self.best_price_qty = Some(new_qty);
            }
            _ => {}
        }
    }

    #[inline]
    fn update_best_price_after_level_delete(&mut self, deleted_price: Px) {
        if self.best_price == Some(deleted_price) {
            if let Some(best_price) = self.levels().keys().max().copied() {
                self.best_price_qty = self.levels().get(&best_price).copied();
                self.best_price = Some(best_price);
            } else {
                self.best_price = None;
                self.best_price_qty = None;
            }
        }
    }

    #[inline]
    fn update_best_price_after_qty_delete(&mut self, deleted_price: Px, deleted_qty: Qty) {
        if self.best_price == Some(deleted_price) {
            self.best_price_qty = self.best_price_qty.map(|qty| qty - deleted_qty);
        }
    }
}

impl<Px: price_level::Price, Qty: QuantityLike> PricePointMutationOps<Px, Qty>
    for BookSideWithBasicTracking<Px, Qty>
{
    #[inline]
    fn add_qty(&mut self, price: Px, qty: Qty) -> FoundLevelType<Qty> {
        let found_level_type = self.find_or_create_level_and_add_qty(price, qty);
        match found_level_type {
            FoundLevelType::New(_) => self.update_best_price_after_add(price, qty),
            FoundLevelType::Existing(new_qty) => self.update_best_price_after_add(price, new_qty),
        };
        found_level_type
    }

    #[inline]
    fn delete_qty(
        &mut self,
        price: Px,
        qty: Qty,
    ) -> Result<DeleteLevelType<Qty>, PricePointMutationOpsError> {
        let current_qty =
            self.levels_mut()
                .get_mut(&price)
                .ok_or(PricePointMutationOpsError::LevelError(
                    LevelError::LevelNotFound,
                ))?;
        match qty.cmp(current_qty) {
            std::cmp::Ordering::Equal => {
                self.levels_mut().remove(&price);
                self.update_best_price_after_level_delete(price);
                Ok(DeleteLevelType::Deleted)
            }
            std::cmp::Ordering::Less => {
                *current_qty -= qty;
                self.update_best_price_after_qty_delete(price, qty);
                Ok(DeleteLevelType::QuantityDecreased(qty))
            }
            std::cmp::Ordering::Greater => Err(PricePointMutationOpsError::QtyExceedsAvailable),
        }
    }
}

impl<Px: price_level::Price, Qty: QuantityLike> PricePointSummaryOps<Px, Qty>
    for BookSideWithBasicTracking<Px, Qty>
{
    fn set_level(&mut self, price: Px, new_qty: Qty) {
        match self.levels.entry(price) {
            hashbrown::hash_map::Entry::Occupied(mut entry) => {
                if new_qty.is_zero() {
                    entry.remove();
                    self.update_best_price_after_level_delete(price);
                } else {
                    entry.insert(new_qty);
                    // This level is occupied, so there is at least one price and best_price is Some(_).
                    if self.best_price.unwrap() == price {
                        self.best_price_qty = Some(new_qty);
                    }
                }
            }
            hashbrown::hash_map::Entry::Vacant(entry) => {
                // TODO: could return an error if zero quantity on a non-existing level.
                if !new_qty.is_zero() {
                    entry.insert(new_qty);
                    self.update_best_price_after_add(price, new_qty);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use order_book_core::price_level;
    use order_book_core::price_level::{AskPrice, BidPrice};

    use super::*;

    fn create_book_side_with_orders<Px>() -> BookSideWithBasicTracking<Px, u32>
    where
        Px: price_level::Price + From<u32>,
    {
        let mut book_side = BookSideWithBasicTracking::new();
        book_side.add_qty(1.into(), 100);
        book_side.add_qty(2.into(), 100);
        book_side.add_qty(3.into(), 101);
        book_side.add_qty(4.into(), 98);
        book_side
    }

    #[test]
    fn test_new() {
        let book_side: BookSideWithBasicTracking<BidPrice<u32>, u32> =
            BookSideWithBasicTracking::new();
        assert_eq!(book_side.levels.len(), 0);

        let book_side: BookSideWithBasicTracking<AskPrice<u32>, u32> =
            BookSideWithBasicTracking::new();
        assert_eq!(book_side.levels.len(), 0);
    }

    #[test]
    fn test_add_qty_to_empty_book() {
        let qty = 5;
        let price = BidPrice(100u32);
        let mut book_side = BookSideWithBasicTracking::new();
        assert_eq!(book_side.best_price, None);
        assert_eq!(book_side.best_price_qty, None);
        book_side.add_qty(price, qty);
        assert_qty_added(&book_side, price, qty, 0, 0);
        assert_eq!(book_side.best_price, Some(price));
        assert_eq!(book_side.best_price_qty, Some(qty));

        let price = AskPrice(100u32);
        let mut book_side = BookSideWithBasicTracking::new();
        assert_eq!(book_side.best_price, None);
        assert_eq!(book_side.best_price_qty, None);
        book_side.add_qty(price, qty);
        assert_qty_added(&book_side, price, qty, 0, 0);
        assert_eq!(book_side.best_price, Some(price));
        assert_eq!(book_side.best_price_qty, Some(qty));
    }

    #[test]
    fn test_add_qty() {
        #[derive(Clone)]
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

        for TestCase { price, qty } in test_cases.clone() {
            let sided_price = BidPrice(price);
            let mut book_side = create_book_side_with_orders();
            let num_levels_before = book_side.levels.len();
            let qty_before = book_side.levels().get(&sided_price).map_or(0, |&q| q);
            book_side.add_qty(sided_price, qty);
            assert_qty_added(&book_side, sided_price, qty, qty_before, num_levels_before);
        }
        for TestCase { price, qty } in test_cases {
            let sided_price = AskPrice(price);
            let mut book_side = create_book_side_with_orders();
            let num_levels_before = book_side.levels.len();
            let qty_before = book_side.levels.get(&sided_price).map_or(0, |&q| q);
            book_side.add_qty(sided_price, qty);
            assert_qty_added(&book_side, sided_price, qty, qty_before, num_levels_before);
        }
    }

    fn assert_qty_added<Px: price_level::Price>(
        book_side: &BookSideWithBasicTracking<Px, u32>,
        price: Px,
        qty: u32,
        qty_before: u32,
        num_levels_before: usize,
    ) {
        let new_level_created = qty_before == 0;
        assert_eq!(
            book_side.levels.len(),
            num_levels_before + new_level_created as usize
        );
        let qty_now = book_side.levels.get(&price).expect("Level not found");
        assert_eq!(*qty_now, qty_before + qty);
    }

    #[test]
    fn test_delete_qty() {
        let mut book_side = BookSideWithBasicTracking::new();
        let (price, qty) = (BidPrice(100), 10);
        book_side.add_qty(price, qty);
        assert_eq!(book_side.best_price, Some(price));
        assert_eq!(book_side.best_price_qty, Some(qty));

        book_side.delete_qty(price, qty).unwrap();
        assert_eq!(book_side.levels.len(), 0);
        assert_eq!(book_side.best_price, None);
        assert_eq!(book_side.best_price_qty, None);

        let mut book_side = BookSideWithBasicTracking::new();
        let (price, qty) = (AskPrice(100), 10);
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
        let mut book_side = BookSideWithBasicTracking::new();
        book_side.add_qty(AskPrice(101), 20);
        assert_eq!(book_side.best_price, Some(AskPrice(101)));
        assert_eq!(book_side.best_price_qty, Some(20));

        book_side.add_qty(AskPrice(100), 10);
        assert_eq!(book_side.best_price, Some(AskPrice(100)));
        assert_eq!(book_side.best_price_qty, Some(10));

        let mut book_side = BookSideWithBasicTracking::new();
        book_side.add_qty(BidPrice(100), 10);
        assert_eq!(book_side.best_price, Some(BidPrice(100)));
        assert_eq!(book_side.best_price_qty, Some(10));

        book_side.add_qty(BidPrice(101), 20);
        assert_eq!(book_side.best_price, Some(BidPrice(101)));
        assert_eq!(book_side.best_price_qty, Some(20));
    }

    #[test]
    fn test_best_price_modify_quantity() {
        let mut book_side = BookSideWithBasicTracking::new();
        book_side.add_qty(BidPrice(100), 10);
        assert_eq!(book_side.best_price, Some(BidPrice(100)));
        assert_eq!(book_side.best_price_qty, Some(10));

        book_side.add_qty(BidPrice(100), 20);
        assert_eq!(book_side.best_price, Some(BidPrice(100)));
        assert_eq!(book_side.best_price_qty, Some(30));

        book_side.delete_qty(BidPrice(100), 15).unwrap();
        assert_eq!(book_side.best_price, Some(BidPrice(100)));
        assert_eq!(book_side.best_price_qty, Some(15));

        book_side.delete_qty(BidPrice(100), 15).unwrap();
        assert_eq!(book_side.best_price, None);
        assert_eq!(book_side.best_price_qty, None);

        let mut book_side = BookSideWithBasicTracking::new();
        book_side.add_qty(AskPrice(100), 10);
        assert_eq!(book_side.best_price, Some(AskPrice(100)));
        assert_eq!(book_side.best_price_qty, Some(10));

        book_side.add_qty(AskPrice(100), 20);
        assert_eq!(book_side.best_price, Some(AskPrice(100)));
        assert_eq!(book_side.best_price_qty, Some(30));

        book_side.delete_qty(AskPrice(100), 15).unwrap();
        assert_eq!(book_side.best_price, Some(AskPrice(100)));
        assert_eq!(book_side.best_price_qty, Some(15));

        book_side.delete_qty(AskPrice(100), 15).unwrap();
        assert_eq!(book_side.best_price, None);
        assert_eq!(book_side.best_price_qty, None);
    }

    #[test]
    fn test_modify_price() {
        let mut book_side = BookSideWithBasicTracking::new();
        book_side.add_qty(BidPrice(100), 10);
        assert_eq!(book_side.best_price, Some(BidPrice(100)));
        assert_eq!(book_side.best_price_qty, Some(10));

        book_side.delete_qty(BidPrice(100), 10).unwrap();
        book_side.add_qty(BidPrice(101), 20);
        assert_eq!(book_side.best_price, Some(BidPrice(101)));
        assert_eq!(book_side.best_price_qty, Some(20));

        book_side.delete_qty(BidPrice(101), 20).unwrap();
        book_side.add_qty(BidPrice(100), 15);
        assert_eq!(book_side.best_price, Some(BidPrice(100)));
        assert_eq!(book_side.best_price_qty, Some(15));
    }

    #[test]
    fn test_set_level() {
        // Test for bid side
        let mut bid_side = BookSideWithBasicTracking::<BidPrice<i64>, i64>::new();

        // Set a new level
        bid_side.set_level(BidPrice(100), 10);
        assert_eq!(bid_side.levels.get(&BidPrice(100)), Some(&10));
        assert_eq!(bid_side.best_price, Some(BidPrice(100)));
        assert_eq!(bid_side.best_price_qty, Some(10));

        // Update existing level
        bid_side.set_level(BidPrice(100), 20);
        assert_eq!(bid_side.levels.get(&BidPrice(100)), Some(&20));
        assert_eq!(bid_side.best_price, Some(BidPrice(100)));
        assert_eq!(bid_side.best_price_qty, Some(20));

        // Add better price
        bid_side.set_level(BidPrice(110), 5);
        assert_eq!(bid_side.levels.get(&BidPrice(110)), Some(&5));
        assert_eq!(bid_side.best_price, Some(BidPrice(110)));
        assert_eq!(bid_side.best_price_qty, Some(5));

        // Remove best price
        bid_side.set_level(BidPrice(110), 0);
        assert_eq!(bid_side.levels.get(&BidPrice(110)), None);
        assert_eq!(bid_side.best_price, Some(BidPrice(100)));
        assert_eq!(bid_side.best_price_qty, Some(20));

        // Test for ask side
        let mut ask_side = BookSideWithBasicTracking::<AskPrice<i64>, i64>::new();

        // Set a new level
        ask_side.set_level(AskPrice(100), 10);
        assert_eq!(ask_side.levels.get(&AskPrice(100)), Some(&10));
        assert_eq!(ask_side.best_price, Some(AskPrice(100)));
        assert_eq!(ask_side.best_price_qty, Some(10));

        // Update existing level
        ask_side.set_level(AskPrice(100), 20);
        assert_eq!(ask_side.levels.get(&AskPrice(100)), Some(&20));
        assert_eq!(ask_side.best_price, Some(AskPrice(100)));
        assert_eq!(ask_side.best_price_qty, Some(20));

        // Add better price
        ask_side.set_level(AskPrice(90), 5);
        assert_eq!(ask_side.levels.get(&AskPrice(90)), Some(&5));
        assert_eq!(ask_side.best_price, Some(AskPrice(90)));
        assert_eq!(ask_side.best_price_qty, Some(5));

        // Remove best price
        ask_side.set_level(AskPrice(90), 0);
        assert_eq!(ask_side.levels.get(&AskPrice(90)), None);
        assert_eq!(ask_side.best_price, Some(AskPrice(100)));
        assert_eq!(ask_side.best_price_qty, Some(20));
    }
}
