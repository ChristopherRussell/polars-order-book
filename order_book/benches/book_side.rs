use criterion::{black_box, Criterion, criterion_group, criterion_main};
use itertools::izip;

use order_book::book_side::BookSide;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut book = black_box(BookSide::new(true));
    let prices = [1i64, 2, 3, 1, 2, 3, 3, 1, 2, 3, 1, 2];
    let quantities = [1i64, 2, 3, 1, 2, 3, -3, -1, -2, -3, -1, -2];

    c.bench_function("book_side_simple", |b| b.iter(|| (black_box({
        for (price, qty) in izip!(prices.into_iter(), quantities.into_iter()) {
            if qty > 0 {
                book.add_qty(price, qty);
            } else {
                book.delete_qty(price, qty.abs()).expect("Deleted more qty than available");
            }
        }
    }))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
