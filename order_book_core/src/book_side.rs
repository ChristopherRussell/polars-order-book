use crate::book_side_ops::{LevelError, PricePointMutationOpsError};
use crate::price_level::{self, QuantityLike};
use hashbrown::HashMap;
use std::fmt::Debug;

#[cfg(not(feature = "tracing"))]
macro_rules! debug { ($($arg:tt)*) => {{}}; }
#[cfg(feature = "tracing")]
use tracing::debug;

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
        let mut candidates: Vec<_> = self
            .levels()
            .iter()
            .map(|(price, qty)| PriceLevel {
                price: *price,
                qty: *qty,
            })
            .collect();
        // select_nth_unstable_by partitions so element at `target` is the one that would
        // appear at that index in a fully sorted array. We want nth from the back (nth best).
        // AskPrice has custom Ord that reverses ordering, so the same logic works for both sides.
        let target = candidates.len().checked_sub(n + 1)?;
        candidates.select_nth_unstable_by(target, |a, b| a.price.cmp(&b.price));
        Some(candidates[target])
    }

    #[cfg_attr(feature = "tracing", tracing::instrument)]
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

    #[cfg_attr(feature = "tracing", tracing::instrument)]
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

    #[cfg_attr(feature = "tracing", tracing::instrument)]
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
