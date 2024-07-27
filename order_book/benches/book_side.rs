use criterion::{black_box, criterion_group, criterion_main, Criterion};
use itertools::izip;

use order_book::book_side::BookSide;
use order_book::book_side_ops::BookSideOps;

pub fn book_side_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("untracked_book_side");
    let mut book = black_box(BookSide::new(true));
    let prices = [1i64, 2, 3, 1, 2, 3, 3, 1, 2, 3, 1, 2];
    let quantities = [1i64, 2, 3, 1, 2, 3, -3, -1, -2, -3, -1, -2];

    group.bench_function("simple", |b| {
        b.iter(|| {
            black_box({
                for (price, qty) in izip!(prices.into_iter(), quantities.into_iter()) {
                    if qty > 0 {
                        book.add_qty(price, qty);
                    } else {
                        book.delete_qty(price, qty.abs())
                            .expect("Deleted more qty than available");
                    }
                }
            })
        })
    });
}

pub fn book_side_performance_by_nr_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("untracked_book_side");
    for is_bid in [true, false] {
        for nr_levels in [1, 100, 10_000] {
            let mut book = black_box(BookSide::new(is_bid));
            let range = 1000..1000 + nr_levels;
            let prices = range.clone().collect::<Vec<i64>>();
            let quantities = range.collect::<Vec<i64>>();
            for (price, qty) in izip!(prices.iter(), quantities.iter()) {
                book.add_qty(*price, *qty);
            }

            let side_name = if is_bid { "bid" } else { "ask" };
            let (best_px, best_qty, next_px, next_qty) = if is_bid {
                let (best_px, best_qty) = (*prices.last().unwrap(), *quantities.last().unwrap());
                (best_px, best_qty, best_px + 1, best_qty + 1)
            } else {
                let (best_px, best_qty) = (prices[0], quantities[0]);
                (best_px, best_qty, best_px - 1, best_qty - 1)
            };
            group.bench_function(
                format!("{}_{}_levels", nr_levels, side_name).as_str(),
                |b| {
                    b.iter(|| {
                        black_box({
                            // Repeatedly modify best px to next px and back
                            for _ in 0..500 {
                                book.delete_qty(best_px, best_qty).unwrap();
                                book.add_qty(next_px, next_qty);
                                book.delete_qty(next_px, next_qty).unwrap();
                                book.add_qty(best_px, best_qty);
                            }
                        })
                    })
                },
            );
        }
    }
}

criterion_group!(
    benches,
    book_side_simple,
    book_side_performance_by_nr_levels
);
criterion_main!(benches);
