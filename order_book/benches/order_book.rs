use criterion::{black_box, criterion_group, criterion_main, Criterion};
use itertools::izip;

use order_book::order_book::OrderBook;

pub fn order_book_simple(c: &mut Criterion) {
    let mut book = black_box(OrderBook::new());
    let prices = [1i64, 2, 3, 6, 5, 4, 3, 1, 2, 5, 4, 6];
    let quantities = [1i64, 2, 3, 6, 5, 4, -3, -1, -2, -5, -4, -6];
    let is_bid = [
        true, true, true, false, false, false, true, true, true, false, false, false,
    ];
    c.bench_function("order_book_simple", |b| {
        b.iter(|| {
            black_box({
                for (price, qty, is_bid) in izip!(
                    prices.into_iter(),
                    quantities.into_iter(),
                    is_bid.into_iter()
                ) {
                    if qty > 0 {
                        book.add_qty(is_bid, price, qty);
                    } else {
                        book.delete_qty(is_bid, price, qty.abs());
                    }
                }
            })
        })
    });
}

criterion_group!(benches, order_book_simple);
criterion_main!(benches);
