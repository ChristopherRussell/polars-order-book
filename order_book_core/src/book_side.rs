use crate::book_side_ops::{LevelError, PricePointMutationOpsError};
use crate::price_level::{self, QuantityLike};
use hashbrown::HashMap;
use itertools::Itertools;
use std::fmt::Debug;
use tracing::{debug, instrument};

use crate::price_level::PriceLevel;

#[derive(Clone, Copy, Debug)]
pub enum FoundLevelType<Qty: QuantityLike> {
    New(Qty),
    Existing(Qty),
}

#[derive(Clone, Copy, Debug)]
pub enum DeleteLevelType<Qty: QuantityLike> {
    Deleted,
    QuantityDecreased(Qty),
}

pub trait BookSide<Px: price_level::Price, Qty: QuantityLike>: Debug {
    // Have considered replacing self.levels HashMap with a BTreeMap, but the slowdown
    // for operations other than getting nth best level does not seem worth it during
    // tracking unless order book has a lot of levels (~1000+)
    fn levels(&self) -> &HashMap<Px, Qty>;
    fn levels_mut(&mut self) -> &mut HashMap<Px, Qty>;

    #[inline]
    fn get_level_qty<'a>(&'a self, price: &'a Px) -> Option<&'a Qty> {
        self.levels().get(price)
    }

    #[inline]
    fn get_level_qty_mut<'a>(&'a mut self, price: &'a Px) -> Option<&'a mut Qty> {
        self.levels_mut().get_mut(price)
    }

    #[inline]
    fn nth_best_level(&self, n: usize) -> Option<PriceLevel<Px, Qty>> {
        let mut sorted = self
            .levels()
            .iter()
            .sorted_unstable_by_key(|(price, _)| *price)
            .map(|(price, qty)| PriceLevel {
                price: *price,
                qty: *qty,
            });
        // AskPrice has custom Ord implementation that reverses the ordering, so logic is same as for Bid.
        sorted.nth_back(n)
    }

    #[instrument]
    #[inline]
    fn find_or_create_level_and_add_qty(&mut self, price: Px, qty: Qty) -> FoundLevelType<Qty> {
        debug!("Adding quantity to book_side");
        match self.levels_mut().entry(price) {
            hashbrown::hash_map::Entry::Occupied(o) => {
                debug!("Updating an existing price level");
                let current_qty = o.into_mut();
                *current_qty += qty;
                FoundLevelType::Existing(*current_qty)
            }
            hashbrown::hash_map::Entry::Vacant(v) => {
                debug!("Created a new price level");
                v.insert(qty);
                FoundLevelType::New(qty)
            }
        }
    }

    #[instrument]
    #[inline]
    fn find_or_create_level_and_set_qty(&mut self, price: Px, qty: Qty) -> FoundLevelType<Qty> {
        debug!("Setting quantity for level");
        match self.levels_mut().entry(price) {
            hashbrown::hash_map::Entry::Occupied(o) => {
                o.replace_entry_with(|_, _| Some(qty));
                FoundLevelType::Existing(qty)
            }
            hashbrown::hash_map::Entry::Vacant(v) => {
                debug!("Created a new price level");
                v.insert(qty);
                FoundLevelType::New(qty)
            }
        }
    }

    #[instrument]
    #[inline]
    fn remove_qty_from_level_and_maybe_delete(
        &mut self,
        price: Px,
        qty: Qty,
    ) -> Result<DeleteLevelType<Qty>, PricePointMutationOpsError> {
        debug!("Deleting quantity from level");
        let current_qty = self
            .levels_mut()
            .get_mut(&price)
            .ok_or(PricePointMutationOpsError::from(LevelError::LevelNotFound))?;
        match qty.cmp(current_qty) {
            std::cmp::Ordering::Equal => {
                _ = self.levels_mut().remove(&price).unwrap();
                Ok(DeleteLevelType::Deleted)
            }
            std::cmp::Ordering::Less => {
                *current_qty -= qty;
                Ok(DeleteLevelType::QuantityDecreased(*current_qty))
            }
            std::cmp::Ordering::Greater => Err(PricePointMutationOpsError::QtyExceedsAvailable),
        }
    }
}
