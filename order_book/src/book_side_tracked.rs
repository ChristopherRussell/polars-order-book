use num::Num;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::Hash;

use crate::book_side::{BookSide, DeleteLevelType, FoundLevelType};
use crate::price_level::PriceLevel;
use crate::top_n_levels::NLevels;

trait BookSideOps<Price, Qty, const N: usize> {
    fn add_qty(&mut self, price: Price, qty: Qty);
    fn modify_qty(&mut self, price: Price, qty: Qty, prev_price: Price, prev_qty: Qty) {
        self.delete_qty(prev_price, prev_qty);
        self.add_qty(price, qty);
    }
    fn delete_qty(&mut self, price: Price, qty: Qty);
    fn top_n(&self) -> &crate::top_n_levels::NLevels<Price, Qty, N>;
}

struct BookSideWithTopNTracking<Price, Qty, const N: usize> {
    book_side: BookSide<Price, Qty>,
    top_n_levels: NLevels<Price, Qty, N>,
}

impl<
        Price: Debug + Eq + Ord + Copy + Hash,
        Qty: Debug + Ord + Clone + Copy + Num,
        const N: usize,
    > BookSideOps<Price, Qty, N> for BookSideWithTopNTracking<Price, Qty, N>
{
    fn add_qty(&mut self, price: Price, qty: Qty) {
        let (
            found_level_type,
            PriceLevel {
                price: added_price,
                qty: added_qty,
            },
        ) = self.book_side.add_qty(price, qty);

        match (
            found_level_type,
            self.book_side.is_bid,
            self.top_n_levels.worst_price.map(|px| px.cmp(&added_price)),
        ) {
            // Adding qty to existing best price
            (FoundLevelType::Existing, _, Some(Ordering::Equal)) => {
                self.top_n_levels.update_qty(added_price, added_qty);
            }
            // New bid pirc is better than current best bid price
            (FoundLevelType::New, true, None | Some(Ordering::Less)) => {
                self.top_n_levels.try_insert_sort(PriceLevel {
                    price: added_price,
                    qty: added_qty,
                })
            }
            // New ask price is better than current best ask price
            (FoundLevelType::New, false, None | Some(Ordering::Greater)) => {
                self.top_n_levels.insert_sort_reversed(PriceLevel {
                    price: added_price,
                    qty: added_qty,
                })
            }
            (FoundLevelType::New, _, Some(Ordering::Equal)) => panic!(
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

    fn delete_qty(&mut self, price: Price, qty: Qty) {
        let (delete_type, level) = self.book_side.delete_qty(price, qty).unwrap();
        match (
            delete_type,
            self.book_side.is_bid,
            self.top_n_levels.worst_price.map(|px| px.cmp(&level.price)),
        ) {
            // Ignore delete at a level below worst tracked price
            (_, true, Some(Ordering::Greater)) | (_, false, Some(Ordering::Less)) => {}
            // Quantity decreased at a tracked level
            (DeleteLevelType::QuantityDecreased, _, Some(Ordering::Equal)) => {
                self.top_n_levels.update_qty(level.price, level.qty);
            }
            // Tracked level delete, find next best level and replace
            (DeleteLevelType::Deleted, _, _) => {
                let best_untracked_level = self.book_side.get_nth_best_level(N);
                self.top_n_levels
                    .replace_sort(level.price, best_untracked_level);
            }
            (DeleteLevelType::QuantityDecreased, _, None) => {
                panic!(
                    "update_best_price_after_delete: If there is an existing level then best price should not be None"
                )
            }
            _ => {}
        }

        // self.top_n_levels.replace(price, qty);
        todo!("Handle errors from book_side, replace or update, handle those errors too")
    }

    fn top_n(&self) -> &NLevels<Price, Qty, N> {
        &self.top_n_levels
    }
}
