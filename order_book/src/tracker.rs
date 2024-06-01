use crate::book_side::BookSide;
use crate::price_level::PriceLevel;

/// Trait for book side operations with top N tracking.
///
/// TopNLevels is an array of Option<PriceLevel> with length N.
/// with None representing that there are less than N levels in
/// total. The array is sorted from best to worst price level.
/// The array is updated on every add_qty and delete_qty operation.
///
/// ??? Should probably track the other prices too, so it's easy to
/// insert the N'th level after deleting one of the top N.
///
/// Adding a new level to top N is easy, just check if the new level
/// is better than the worst level in top N, if it is, replace the
/// worst level.
///
/// ??? BookSideOpsWithTopNTracking ... do I need ths or just BookSideOps
/// implemented on different structs (BookSide and BookSideWithTopNTracking)?

trait BookSideOps<Price, Qty, const N: usize> {
    fn add_qty(&mut self, price: Price, qty: Qty);
    fn modify_qty(&mut self, price: Price, qty: Qty, prev_price: Price, prev_qty: Qty) {
        self.delete_qty(prev_price, prev_qty);
        self.add_qty(price, qty);
    }
    fn delete_qty(&mut self, price: Price, qty: Qty);
    fn top_n(&self) -> &TopNLevels<Price, Qty, N>;
}

struct BookSideWithTopNTracking<Price, Qty, const N: usize> {
    book_side: BookSide<Price, Qty>,
    top_n_levels: TopNLevels<Price, Qty, N>,
}

struct NLevels<Price, Qty, const N: usize> {
    levels: [Option<PriceLevel<Price, Qty>>; N],
}

enum TopNLevels<Price, Qty, const N: usize> {
    Bids(NLevels<Price, Qty, N>),
    Asks(NLevels<Price, Qty, N>),
}

impl<Price, Qty, const N: usize> TopNLevels<Price, Qty, N> {
    fn new(is_bid: bool) -> Self {
        TopNLevels {
            is_bid,
            levels: core::array::from_fn(|_| None), // Avoids PriceLevel requiring Copy trait
        }
    }
}


impl<Price, Qty, const N: usize> TopNLevels<Price, Qty, N>::Bids {
    fn maybe_add_level(&mut self, level: PriceLevel<Price, Qty>) {
        // 1) if space, add level
        // 2) else if level is better than worst level, replace worst level
        // 3) else do nothing
        // 4) if added something, then sort levels
        todo!()
    }

    fn maybe_delete_level(&mut self, level: PriceLevel<Price, Qty>) {
        // Performance Idea: finding next level to insert may be a bottleneck, might it be a good
        // hueristic to maintain a longer list of levels than N so that we only need to find the
        // next level if several deletes happen consecutively on top of book? In typical order books
        // there are more frequent changes to top of book than deeper in the book.
        todo!()
    }
}

impl<Price, Qty, const N: usize> TopNLevels<Price, Qty, N>::Asks {}