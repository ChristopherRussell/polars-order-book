#![allow(clippy::unit_arg)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use itertools::izip;

use order_book::book_side_simple::SimpleBookSide;
use order_book_core::book_side_ops::PricePointMutationOps;
use order_book_core::price_level::{AskPrice, BidPrice, Price};

pub fn book_side_simple(c: &mut Criterion) {
    let mut book = black_box(SimpleBookSide::new());
    let prices = [1i64, 2, 3, 1, 2, 3, 3, 1, 2, 3, 1, 2].map(BidPrice);
    let quantities = [1i64, 2, 3, 1, 2, 3, -3, -1, -2, -3, -1, -2];

    c.bench_function("untracked_simple", |b| {
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

    for nr_levels in [1, 100, 10_000] {
        side_performance_by_nr_levels::<BidPrice<i64>>(nr_levels, &mut group);
        side_performance_by_nr_levels::<AskPrice<i64>>(nr_levels, &mut group);
    }
}

fn side_performance_by_nr_levels<Px: Price + From<i64> + std::ops::Add<Output = Px>>(
    nr_levels: i64,
    group: &mut criterion::BenchmarkGroup<criterion::measurement::WallTime>,
) {
    let mut book = SimpleBookSide::new();
    let range = 1000..1000 + nr_levels;
    let prices = range.clone().map(|x| Px::from(x)).collect::<Vec<Px>>();
    let quantities = range.collect::<Vec<i64>>();
    for (price, qty) in izip!(prices.iter(), quantities.iter()) {
        book.add_qty(*price, *qty);
    }

    let side_name = Px::SIDE_NAME;
    let is_bid = Px::IS_BID;
    let (best_px, best_qty, next_px, next_qty) = if is_bid {
        let (best_px, best_qty) = (*prices.last().unwrap(), *quantities.last().unwrap());
        (best_px, best_qty, best_px + Px::from(1), best_qty + 1)
    } else {
        let (best_px, best_qty) = (prices[0], quantities[0]);
        (best_px, best_qty, best_px + Px::from(-1), best_qty - 1)
    };
    group.bench_function(BenchmarkId::new(side_name, nr_levels), |b| {
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
    });
}

criterion_group!(
    benches,
    book_side_simple,
    book_side_performance_by_nr_levels
);
criterion_main!(benches);
