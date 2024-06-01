use criterion::{black_box, Criterion, criterion_group, criterion_main};
use itertools::izip;
use polars::io::SerReader;
use polars::prelude::CsvReader;

use order_book::order_book::OrderBook;

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut book: OrderBook<i64, i64> = black_box(OrderBook::new());
    let data = CsvReader::from_path("ninja_order_book.csv").unwrap().finish().unwrap();
    let prices = data.column("price").unwrap().i64().unwrap();
    let quantities = data.column("qty_diff").unwrap().i64().unwrap();
    let is_bids = data.column("is_bid").unwrap().bool().unwrap();
    let data = izip!(is_bids, prices, quantities).map(
        |(f, p, q)| (f.unwrap(), p.unwrap(), q.unwrap())).collect::<Vec<(bool, i64, i64)>>();

    c.bench_function("book_side_simple", |b| b.iter(|| (black_box({
        for (is_bid, price, qty) in data.clone() {
            if qty < 0 {
                book.delete_qty(is_bid, price, qty.abs());
            } else {
                book.add_qty(is_bid, price, qty);
            }
        }
    }))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
