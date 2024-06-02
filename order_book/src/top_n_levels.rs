use crate::price_level::PriceLevel;

/// Trait for book side operations with top N tracking.
///
/// TopNLevels is an array of Option<PriceLevel> with length N.
/// with None representing that there are less than N levels in
/// total. The array is sorted from best to worst price level.
/// The array is updated on every add_qty and delete_qty operation.
///
/// ??? Should probably track the other prices too, so it's easy to
/// insert the Nth level after deleting one of the top N.
///
/// Adding a new level to top N is easy, just check if the new level
/// is better than the worst level in top N, if it is, replace the
/// worst level.
///
/// ??? BookSideOpsWithTopNTracking ... do I need ths or just BookSideOps
/// implemented on different structs (BookSide and BookSideWithTopNTracking)?


pub struct NLevels<Price, Qty, const N: usize> {
    levels: [Option<PriceLevel<Price, Qty>>; N],
    worst_price: Option<Price>,
}

pub enum TopNLevels<Price, Qty, const N: usize> {
    Bids(NLevels<Price, Qty, N>),
    Asks(NLevels<Price, Qty, N>),
}

impl<Price, Qty, const N: usize> NLevels<Price, Qty, N> {
    fn new() -> Self {
        Self::default()
    }

    fn default() -> Self {
        assert!(N > 0, "TopNLevels: N must be greater than 0");
        NLevels {
            levels: core::array::from_fn(|_| None), // Avoids PriceLevel requiring Copy trait
            worst_price: None,
        }
    }
}

impl<Price: PartialOrd + Clone + Copy + std::fmt::Debug, Qty: std::fmt::Debug, const N: usize> TopNLevels<Price, Qty, N> {
    fn levels(&self) -> &[Option<PriceLevel<Price, Qty>>] {
        match self {
            TopNLevels::Bids(NLevels { levels, .. }) => levels,
            TopNLevels::Asks(NLevels { levels, .. }) => levels,
        }
    }

    fn worst_price(&self) -> Option<Price> {
        match self {
            TopNLevels::Bids(NLevels { worst_price, .. }) => *worst_price,
            TopNLevels::Asks(NLevels { worst_price, .. }) => *worst_price,
        }
    }
    fn maybe_add_level(&mut self, new_level: PriceLevel<Price, Qty>) {
        // alternative, simpler: put the level at end after checking last price,
        // then find the insertion point, create a mut slice, and rotate_right
        let n_levels = match self {
            TopNLevels::Bids(n_levels) => n_levels,
            TopNLevels::Asks(n_levels) => n_levels,
        };

        if let Some(worst_price) = n_levels.worst_price {
            if new_level.price < worst_price {
                return;
            }
        }
        let new_price = new_level.price;
        n_levels.levels[N - 1] = Some(new_level);
        let mut insertion_point = None;
        for (i, entry) in n_levels.levels[..N - 1].iter().enumerate() {
            match entry {
                Some(level) if new_price < level.price => {}
                _ => {
                    insertion_point = Some(i);
                    break;
                }
            }
        }
        if let Some(insertion_point) = insertion_point {
            n_levels.levels[insertion_point..].rotate_right(1);
        }
    }

    fn add_level(&mut self, new_level: PriceLevel<Price, Qty>) {
        // Assume that add_level is only called when new level is better than worst level.
        // Or when there are None entries, since those can always be filled.
        // heuristic: iterate prices from best to worst when finding insertion point since
        // orders books tend to be updated more frequently on top of book than lower levels.

        // find insertion point and swap with value
        match self {
            TopNLevels::Bids(n_levels) => {
                let mut levels = n_levels.levels[..N - 1].iter_mut();
                let mut price_to_insert = new_level;
                loop {
                    match levels.next() {
                        Some(Some(level)) => {
                            // TODO - optimization: no need to check last level, just insert
                            if price_to_insert.price > level.price {
                                std::mem::swap(&mut price_to_insert, level);
                                break;
                            }
                        }
                        Some(entry) => {
                            *entry = Some(price_to_insert);
                            return; // worst price is None and stays None, also no need to shift Nones.
                        }
                        None => unsafe {
                            // Assumed that add_level is only called when new level is better than worst level
                            // 1) safe to unwrap last since levels has length N > 0
                            n_levels.worst_price = Some(price_to_insert.price);
                            *n_levels.levels.last_mut().unwrap_unchecked() = Some(price_to_insert);
                            return;
                        }
                    }
                }

                // Continue swapping values until end of array, or None entry. Also update worst price.
                loop {  // no longer need to identify insertion point, just swap until end or None entry
                    match levels.next() {
                        // TODO - optimization: no need to swap for the last position, just insert.
                        Some(Some(level)) => {
                            std::mem::swap(&mut price_to_insert, level);
                        }
                        Some(entry) => {
                            if levels.next().is_none() {
                                n_levels.worst_price = Some(price_to_insert.price);
                            }
                            *entry = Some(price_to_insert);
                            break;
                        }
                        None => {
                            n_levels.worst_price = Some(price_to_insert.price);
                            unsafe {
                                *n_levels.levels.last_mut().unwrap_unchecked() = Some(price_to_insert);
                                break;
                            }
                        }
                    }
                }
            }
            TopNLevels::Asks(n_levels) => {
                let mut levels = n_levels.levels[..N - 1].iter_mut();
                let mut price_to_insert = new_level;
                loop {
                    match levels.next() {
                        Some(Some(level)) => {
                            // TODO - this price comparison is the only difference with the Bids branch
                            // can we reduce duplicated code without a performance cost?
                            if price_to_insert.price < level.price {
                                std::mem::swap(&mut price_to_insert, level);
                                break;
                            }
                        }
                        Some(entry) => {
                            *entry = Some(price_to_insert);
                            return;
                        }
                        None => unsafe {
                            n_levels.worst_price = Some(price_to_insert.price);
                            *n_levels.levels.last_mut().unwrap_unchecked() = Some(price_to_insert);
                            return;
                        }
                    }
                }
                loop {
                    match levels.next() {
                        Some(Some(level)) => {
                            std::mem::swap(&mut price_to_insert, level);
                        }
                        Some(entry) => {
                            if levels.next().is_none() {
                                n_levels.worst_price = Some(price_to_insert.price);
                            }
                            *entry = Some(price_to_insert);
                            break;
                        }
                        None => {
                            n_levels.worst_price = Some(price_to_insert.price);
                            unsafe {
                                *n_levels.levels.last_mut().unwrap_unchecked() = Some(price_to_insert);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    fn delete_and_replace(&mut self, price_to_delete: Price, next_best_level: Option<PriceLevel<Price, Qty>>) {
        // 1) find level to delete (take)
        // 2) shift other levels until find new level entry point
        // 3) insert new level

        todo!()
    }
    fn maybe_delete_level(&mut self, price_to_delete: Price, new_level: PriceLevel<Price, Qty>) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_n_levels_constructor() {
        let n_levels: NLevels<i32, i32, 5> = NLevels::default();
        assert_eq!(n_levels.levels.len(), 5);
        assert!(n_levels.levels.iter().all(|level| level.is_none()));
        assert_eq!(n_levels.worst_price, None);
    }

    #[test]
    fn test_add_level() {
        let mut top_n = TopNLevels::Bids(NLevels::<i32, i32, 5>::default());
        let level = PriceLevel::new(true, 1);
        top_n.add_level(level);
        assert_eq!(top_n.levels()[0], Some(level));
    }

    fn get_price_levels(is_bid: bool, prices: [i32; 5]) -> [Option<PriceLevel<i32, i32>>; 5] {
        prices.map(|price| Some(PriceLevel::new(is_bid, price)))
    }

    fn get_full_top_n_bids() -> TopNLevels<i32, i32, 5> {
        let mut top_n = TopNLevels::Bids(NLevels::<i32, i32, 5>::default());
        for i in 1..6 {
            let level = PriceLevel::new(true, i * 2);
            top_n.add_level(level);
        }
        assert_eq!(top_n.levels(), get_price_levels(true, [10, 8, 6, 4, 2]));
        top_n
    }

    fn get_full_top_n_asks() -> TopNLevels<i32, i32, 5> {
        let mut top_n = TopNLevels::Asks(NLevels::<i32, i32, 5>::default());
        for i in 1..6 {
            let level = PriceLevel::new(false, i * 2);
            top_n.add_level(level);
        }
        assert_eq!(top_n.levels(), get_price_levels(false, [2, 4, 6, 8, 10]));
        top_n
    }

    #[test]
    fn test_add_level_when_not_full() {
        let mut top_n = TopNLevels::Bids(NLevels::<i32, i32, 2>::default());
        let level: PriceLevel<i32, i32> = PriceLevel::new(true, 1);
        top_n.add_level(level);
        assert_eq!(top_n.worst_price(), None);
        assert_eq!(top_n.levels(), [Some(PriceLevel::new(true, 1)), None]);

        top_n.add_level(PriceLevel::new(true, 2));
        assert_eq!(top_n.worst_price(), Some(1));
        assert_eq!(top_n.levels(), [Some(PriceLevel::new(true, 2)), Some(PriceLevel::new(true, 1))]);
    }

    #[test]
    fn test_add_level_when_full() {
        let mut top_n = get_full_top_n_bids();
        let level: PriceLevel<i32, i32> = PriceLevel::new(true, 12);
        top_n.add_level(level);
        assert_eq!(top_n.levels(), get_price_levels(true, [12, 10, 8, 6, 4]));
        assert_eq!(top_n.worst_price(), Some(4));

        let mut top_n = get_full_top_n_bids();
        let level: PriceLevel<i32, i32> = PriceLevel::new(true, 5);
        top_n.add_level(level);
        assert_eq!(top_n.levels(), get_price_levels(true, [10, 8, 6, 5, 4]));
        assert_eq!(top_n.worst_price(), Some(4));

        let mut top_n = get_full_top_n_bids();
        let level: PriceLevel<i32, i32> = PriceLevel::new(true, 3);
        top_n.add_level(level);
        assert_eq!(top_n.levels(), get_price_levels(true, [10, 8, 6, 4, 3]));
        assert_eq!(top_n.worst_price(), Some(3));

        let mut top_n = get_full_top_n_asks();
        let level: PriceLevel<i32, i32> = PriceLevel::new(false, 1);
        top_n.add_level(level);
        assert_eq!(top_n.levels(), get_price_levels(false, [1, 2, 4, 6, 8]));
        assert_eq!(top_n.worst_price(), Some(8));

        let mut top_n = get_full_top_n_asks();
        let level: PriceLevel<i32, i32> = PriceLevel::new(false, 3);
        top_n.add_level(level);
        assert_eq!(top_n.levels(), get_price_levels(false, [2, 3, 4, 6, 8]));
        assert_eq!(top_n.worst_price(), Some(8));

        let mut top_n = get_full_top_n_asks();
        let level: PriceLevel<i32, i32> = PriceLevel::new(false, 9);
        top_n.add_level(level);
        assert_eq!(top_n.levels(), get_price_levels(false, [2, 4, 6, 8, 9]));
        assert_eq!(top_n.worst_price(), Some(9));
    }

    #[test]
    fn test_level_below_worst_inserts_incorrectly() {
        let mut top_n = get_full_top_n_bids();
        let level: PriceLevel<i32, i32> = PriceLevel::new(true, 1);
        top_n.add_level(level);
        assert_eq!(top_n.levels(), get_price_levels(true, [10, 8, 6, 4, 1]));
        assert_eq!(top_n.worst_price(), Some(1));

        let mut top_n = get_full_top_n_asks();
        let level: PriceLevel<i32, i32> = PriceLevel::new(false, 12);
        top_n.add_level(level);
        assert_eq!(top_n.levels(), get_price_levels(false, [2, 4, 6, 8, 12]));
        assert_eq!(top_n.worst_price(), Some(12));
    }
}
