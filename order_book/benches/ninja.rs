#![allow(clippy::unit_arg)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use itertools::izip;
use polars::io::SerReader;
use polars::prelude::CsvReadOptions;
use std::path::PathBuf;

use order_book::order_book_simple::SimpleOrderBook;
use order_book::order_book_tracked::OrderBookWithTopNTracking;
use order_book::order_book_tracked_basic::OrderBookWithBasicTracking;
use order_book_core::order_book::PricePointMutationBookOps;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("ninja_order_book");

    let mut book_untracked: SimpleOrderBook<i64, i64> = black_box(SimpleOrderBook::new());
    let mut book_basic: OrderBookWithBasicTracking<i64, i64> =
        black_box(OrderBookWithBasicTracking::new());
    let mut book_1: OrderBookWithTopNTracking<i64, i64, 1> =
        black_box(OrderBookWithTopNTracking::new());
    let mut book_2: OrderBookWithTopNTracking<i64, i64, 2> =
        black_box(OrderBookWithTopNTracking::new());
    let mut book_5: OrderBookWithTopNTracking<i64, i64, 5> =
        black_box(OrderBookWithTopNTracking::new());

    let mut test_data_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_data_path.push("benches/ninja_order_book.csv");
    let data = CsvReadOptions::default()
        .try_into_reader_with_file_path(Some(test_data_path))
        .unwrap()
        .finish()
        .unwrap();
    let prices = data.column("price").unwrap().i64().unwrap();
    let quantities = data.column("qty_diff").unwrap().i64().unwrap();
    let is_bids = data.column("is_bid").unwrap().bool().unwrap();
    let data = izip!(is_bids, prices, quantities)
        .map(|(f, p, q)| (f.unwrap(), p.unwrap(), q.unwrap()))
        .collect::<Vec<(bool, i64, i64)>>();

    group.bench_function("untracked", |b| {
        b.iter(|| {
            let data_cloned = data.clone();
            black_box({
                for (is_bid, price, qty) in data_cloned {
                    if qty < 0 {
                        book_untracked
                            .delete_qty(is_bid, price, i64::abs(qty))
                            .unwrap();
                    } else {
                        book_untracked.add_qty(is_bid, price, qty);
                    }
                }
            })
        })
    });

    group.bench_function("basic_tracked", |b| {
        b.iter(|| {
            let data_cloned = data.clone();
            black_box({
                for (is_bid, price, qty) in data_cloned {
                    if qty < 0 {
                        book_basic.delete_qty(is_bid, price, i64::abs(qty)).unwrap();
                    } else {
                        book_basic.add_qty(is_bid, price, qty);
                    }
                }
            })
        })
    });

    group.bench_function("1_level_tracked", |b| {
        b.iter(|| {
            let data_cloned = data.clone();
            black_box({
                for (is_bid, price, qty) in data_cloned {
                    if qty < 0 {
                        book_1.delete_qty(is_bid, price, i64::abs(qty)).unwrap();
                    } else {
                        book_1.add_qty(is_bid, price, qty);
                    }
                }
            })
        })
    });

    group.bench_function("2_levels_tracked", |b| {
        b.iter(|| {
            let data_cloned = data.clone();
            black_box({
                for (is_bid, price, qty) in data_cloned {
                    if qty < 0 {
                        book_2.delete_qty(is_bid, price, i64::abs(qty)).unwrap();
                    } else {
                        book_2.add_qty(is_bid, price, qty);
                    }
                }
            })
        })
    });

    group.bench_function("5_levels_tracked", |b| {
        b.iter(|| {
            let data_cloned = data.clone();
            black_box({
                for (is_bid, price, qty) in data_cloned {
                    if qty < 0 {
                        book_5.delete_qty(is_bid, price, i64::abs(qty)).unwrap();
                    } else {
                        book_5.add_qty(is_bid, price, qty);
                    }
                }
            })
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
