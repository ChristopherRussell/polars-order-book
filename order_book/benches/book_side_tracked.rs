#![allow(clippy::unit_arg)]
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use itertools::izip;

use order_book::book_side_tracked::BookSideWithTopNTracking;
use order_book::book_side_tracked_basic::BookSideWithBasicTracking;
use order_book_core::book_side_ops::PricePointMutationOps;
use order_book_core::price_level::BidPrice;

pub fn tracked_book_side_performance_by_nr_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("tracked_book_side_performance_by_nr_levels");
    for nr_levels in [1, 100, 10_000] {
        let mut book_1_basic: BookSideWithBasicTracking<BidPrice<i64>, i64> =
            black_box(BookSideWithBasicTracking::new());

        let mut book_1: BookSideWithTopNTracking<BidPrice<i64>, i64, 2> =
            black_box(BookSideWithTopNTracking::new());

        let mut book_2: BookSideWithTopNTracking<BidPrice<i64>, i64, 2> =
            black_box(BookSideWithTopNTracking::new());

        let mut book_5: BookSideWithTopNTracking<BidPrice<i64>, i64, 5> =
            black_box(BookSideWithTopNTracking::new());

        let range = 1000..1000 + nr_levels;
        let prices = range.clone().collect::<Vec<i64>>();
        let quantities = range.collect::<Vec<i64>>();
        for (&price, qty) in izip!(prices.iter(), quantities.iter()) {
            book_1_basic.add_qty(price.into(), *qty);
        }
        for (&price, qty) in izip!(prices.iter(), quantities.iter()) {
            book_1.add_qty(price.into(), *qty);
        }

        for (&price, qty) in izip!(prices.iter(), quantities.iter()) {
            book_2.add_qty(price.into(), *qty);
        }
        for (&price, qty) in izip!(prices.iter(), quantities.iter()) {
            book_5.add_qty(price.into(), *qty);
        }

        let (best_px, best_qty) = (*prices.last().unwrap(), *quantities.last().unwrap());
        let (next_px, next_qty) = (best_px + 1, best_qty + 1);

        group.bench_function(BenchmarkId::new("basic_1_level", nr_levels), |b| {
            b.iter(|| {
                black_box({
                    // Repeatedly modify best px to next px and back
                    for _ in 0..100 {
                        book_1_basic.delete_qty(best_px.into(), best_qty).unwrap();
                        book_1_basic.add_qty(next_px.into(), next_qty);
                        book_1_basic.delete_qty(next_px.into(), next_qty).unwrap();
                        book_1_basic.add_qty(best_px.into(), best_qty);
                    }
                })
            })
        });

        group.bench_function(BenchmarkId::new("1_levels", nr_levels), |b| {
            b.iter(|| {
                black_box({
                    // Repeatedly modify best px to next px and back
                    for _ in 0..100 {
                        book_1.delete_qty(best_px.into(), best_qty).unwrap();
                        book_1.add_qty(next_px.into(), next_qty);
                        book_1.delete_qty(next_px.into(), next_qty).unwrap();
                        book_1.add_qty(best_px.into(), best_qty);
                    }
                })
            })
        });

        group.bench_function(BenchmarkId::new("2_levels", nr_levels), |b| {
            b.iter(|| {
                black_box({
                    // Repeatedly modify best px to next px and back
                    for _ in 0..100 {
                        book_2.delete_qty(best_px.into(), best_qty).unwrap();
                        book_2.add_qty(next_px.into(), next_qty);
                        book_2.delete_qty(next_px.into(), next_qty).unwrap();
                        book_2.add_qty(best_px.into(), best_qty);
                    }
                })
            })
        });
        group.bench_function(BenchmarkId::new("5_levels", nr_levels), |b| {
            b.iter(|| {
                black_box({
                    // Repeatedly modify best px to next px and back
                    for _ in 0..100 {
                        book_5.delete_qty(best_px.into(), best_qty).unwrap();
                        book_5.add_qty(next_px.into(), next_qty);
                        book_5.delete_qty(next_px.into(), next_qty).unwrap();
                        book_5.add_qty(best_px.into(), best_qty);
                    }
                })
            })
        });
    }
}

criterion_group!(benches, tracked_book_side_performance_by_nr_levels);
criterion_main!(benches);
