use crate::book_side::BookSide;

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
    top_n_levels: crate::top_n_levels::NLevels<Price, Qty, N>,
}
