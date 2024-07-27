#![allow(clippy::unused_unit)]

use crate::basic_tracking::{
    calculate_bbo_from_simple_mutations_basic_tracking, calculate_bbo_with_modifies_basic_tracking,
};
use itertools::izip;
use order_book::{book_side_ops::BookSideOps, order_book_tracked::OrderBookWithTopNTracking};
use polars::datatypes::BooleanType;
use polars::prelude::*;
use pyo3_polars::derive::polars_expr;
use serde::Deserialize;
use tracing::{debug, instrument};

#[derive(Deserialize)]
struct CalculateBboKwargs {
    n: usize,
}

fn bbo_struct(input_fields: &[Field], kwargs: CalculateBboKwargs) -> PolarsResult<Field> {
    let price_field = &input_fields[0];
    let qty_field = &input_fields[1];
    let n = kwargs.n;
    let bbo_struct = DataType::Struct(vec![
        Field::new(
            "bid_px",
            DataType::Array(Box::new(price_field.data_type().clone()), n),
        ),
        Field::new(
            "bid_qty",
            DataType::Array(Box::new(qty_field.data_type().clone()), n),
        ),
        Field::new(
            "ask_px",
            DataType::Array(Box::new(price_field.data_type().clone()), n),
        ),
        Field::new(
            "ask_qty",
            DataType::Array(Box::new(qty_field.data_type().clone()), n),
        ),
    ]);
    Ok(Field::new("bbo", bbo_struct))
}

#[polars_expr(output_type_func_with_kwargs = bbo_struct)]
pub fn pl_calculate_bbo(inputs: &[Series], kwargs: CalculateBboKwargs) -> PolarsResult<Series> {
    _pl_calculate_bbo(inputs, kwargs.n)
}

fn _pl_calculate_bbo(inputs: &[Series], n: usize) -> PolarsResult<Series> {
    match inputs.len() {
        3 | 5 => {}
        _ => {
            let input_names = inputs
                .iter()
                .map(|s| s.name())
                .collect::<Vec<&str>>()
                .join(", ");
            panic!(
                "Expected 3 or 5 input columns: price, qty, is_bid, (prev_price, prev_qty)
                but got {} columns called:\n    {}",
                inputs.len(),
                input_names
            )
        }
    }

    let price = inputs[0].i64()?;
    let qty = inputs[1].i64()?;
    let is_bid = inputs[2].bool()?;
    let prev_price = inputs.get(3);
    let prev_qty = inputs.get(4);

    match n {
        1 => match (prev_price, prev_qty) {
            (Some(prev_price), Some(prev_qty)) => {
                let prev_price_chunked = prev_price.i64()?;
                let prev_qty_chunked = prev_qty.i64()?;
                calculate_bbo_with_modifies_basic_tracking(
                    price,
                    qty,
                    is_bid,
                    prev_price_chunked,
                    prev_qty_chunked,
                )
            }
            (None, None) => calculate_bbo_from_simple_mutations_basic_tracking(price, qty, is_bid),
            _ => panic!(
                "Expected both prev_price and prev_qty or neither, got: {:?} and {:?}",
                prev_price, prev_qty
            ),
        },
        2 => match (prev_price, prev_qty) {
            (Some(prev_price), Some(prev_qty)) => {
                let prev_price_chunked = prev_price.i64()?;
                let prev_qty_chunked = prev_qty.i64()?;
                calculate_bbo_with_modifies::<2>(
                    price,
                    qty,
                    is_bid,
                    prev_price_chunked,
                    prev_qty_chunked,
                )
            }
            (None, None) => calculate_bbo_from_simple_mutations::<2>(price, qty, is_bid),
            _ => panic!(
                "Expected both prev_price and prev_qty or neither, got: {:?} and {:?}",
                prev_price, prev_qty
            ),
        },
        3 => match (prev_price, prev_qty) {
            (Some(prev_price), Some(prev_qty)) => {
                let prev_price_chunked = prev_price.i64()?;
                let prev_qty_chunked = prev_qty.i64()?;
                calculate_bbo_with_modifies::<3>(
                    price,
                    qty,
                    is_bid,
                    prev_price_chunked,
                    prev_qty_chunked,
                )
            }
            (None, None) => calculate_bbo_from_simple_mutations::<3>(price, qty, is_bid),
            _ => panic!(
                "Expected both prev_price and prev_qty or neither, got: {:?} and {:?}",
                prev_price, prev_qty
            ),
        },
        4 => match (prev_price, prev_qty) {
            (Some(prev_price), Some(prev_qty)) => {
                let prev_price_chunked = prev_price.i64()?;
                let prev_qty_chunked = prev_qty.i64()?;
                calculate_bbo_with_modifies::<4>(
                    price,
                    qty,
                    is_bid,
                    prev_price_chunked,
                    prev_qty_chunked,
                )
            }
            (None, None) => calculate_bbo_from_simple_mutations::<4>(price, qty, is_bid),
            _ => panic!(
                "Expected both prev_price and prev_qty or neither, got: {:?} and {:?}",
                prev_price, prev_qty
            ),
        },
        _ => panic!("Unsupported N: {}", n),
    }
}

struct BboBuilder<const N: usize> {
    best_ask_prices: [PrimitiveChunkedBuilder<Int64Type>; N],
    best_ask_qtys: [PrimitiveChunkedBuilder<Int64Type>; N],
    best_bid_prices: [PrimitiveChunkedBuilder<Int64Type>; N],
    best_bid_qtys: [PrimitiveChunkedBuilder<Int64Type>; N],
}

impl<const N: usize> BboBuilder<N> {
    fn new(capacity: usize) -> Self {
        let best_ask_prices = core::array::from_fn(|i| {
            PrimitiveChunkedBuilder::new(format!("ask_price_{}", i + 1).as_str(), capacity)
        });
        let best_ask_qtys = core::array::from_fn(|i| {
            PrimitiveChunkedBuilder::new(format!("ask_qty_{}", i + 1).as_str(), capacity)
        });
        let best_bid_prices = core::array::from_fn(|i| {
            PrimitiveChunkedBuilder::new(format!("bid_price_{}", i + 1).as_str(), capacity)
        });
        let best_bid_qtys = core::array::from_fn(|i| {
            PrimitiveChunkedBuilder::new(format!("bid_qty_{}", i + 1).as_str(), capacity)
        });

        Self {
            best_ask_prices,
            best_ask_qtys,
            best_bid_prices,
            best_bid_qtys,
        }
    }

    fn finish(self) -> PolarsResult<DataFrame> {
        let columns = self
            .best_bid_prices
            .into_iter()
            .chain(self.best_bid_qtys)
            .chain(self.best_ask_prices)
            .chain(self.best_ask_qtys)
            .map(|builder| builder.finish().into_series())
            .collect();
        DataFrame::new(columns)
    }

    #[instrument(skip(self))]
    fn update_builders(&mut self, book: &mut OrderBookWithTopNTracking<i64, i64, N>) {
        for (
            bid_px_builder,
            bid_qty_builder,
            bid_level,
            ask_px_builder,
            ask_qty_builder,
            ask_level,
        ) in izip!(
            &mut self.best_bid_prices,
            &mut self.best_bid_qtys,
            book.bids.top_n().iter(),
            &mut self.best_ask_prices,
            &mut self.best_ask_qtys,
            book.offers.top_n().iter(),
        ) {
            if let Some(level) = bid_level {
                bid_px_builder.append_value(level.price);
                bid_qty_builder.append_value(level.qty);
            } else {
                bid_px_builder.append_null();
                bid_qty_builder.append_null();
            }
            if let Some(level) = ask_level {
                ask_px_builder.append_value(level.price);
                ask_qty_builder.append_value(level.qty);
            } else {
                ask_px_builder.append_null();
                ask_qty_builder.append_null();
            }
        }
        debug!("Updated builders");
    }
}

/// Calculate the best bid and best ask prices and quantities
/// using price-point add and delete mutations.
#[instrument]
fn calculate_bbo_from_simple_mutations<const N: usize>(
    price_array: &ChunkedArray<Int64Type>,
    qty_array: &ChunkedArray<Int64Type>,
    is_bid_array: &ChunkedArray<BooleanType>,
) -> PolarsResult<Series> {
    let length = price_array.len();
    let mut bbo_builder = BboBuilder::<N>::new(length);
    let mut book: OrderBookWithTopNTracking<i64, i64, N> = OrderBookWithTopNTracking::default();
    for tuple in izip!(
        is_bid_array.into_iter(),
        price_array.into_iter(),
        qty_array.into_iter(),
    ) {
        if let (Some(is_bid), Some(price), Some(qty)) = tuple {
            apply_simple_mutation(&mut book, is_bid, price, qty);
        } else {
            panic!("Invalid input tuple: {:?}", tuple);
        }
        bbo_builder.update_builders(&mut book)
    }
    let result = bbo_builder.finish()?.into_struct("bbo").into_series();
    Ok(result)
}

/// Calculate the best bid and best ask prices and quantities
/// using price-point mutations which may include modifies, i.e.
/// delete and an add operation in a single row.
#[instrument]
fn calculate_bbo_with_modifies<const N: usize>(
    price_array: &ChunkedArray<Int64Type>,
    qty_array: &ChunkedArray<Int64Type>,
    is_bid_array: &ChunkedArray<BooleanType>,
    prev_price_array: &ChunkedArray<Int64Type>,
    prev_qty_array: &ChunkedArray<Int64Type>,
) -> PolarsResult<Series> {
    let length = price_array.len();
    let mut bbo_builder = BboBuilder::new(length);
    let mut book = OrderBookWithTopNTracking::<i64, i64, N>::default();
    for tuple in izip!(
        is_bid_array.into_iter(),
        price_array.into_iter(),
        qty_array.into_iter(),
        prev_price_array.into_iter(),
        prev_qty_array.into_iter()
    ) {
        match tuple {
            (Some(is_bid), Some(price), Some(qty), None, None) => {
                apply_simple_mutation(&mut book, is_bid, price, qty);
            }
            (Some(is_bid), Some(price), Some(qty), Some(prev_price), Some(prev_qty)) => {
                book.modify_qty(is_bid, prev_price, prev_qty, price, qty)
            }
            (Some(is_bid), Some(price), Some(qty), None, Some(prev_qty)) => {
                apply_simple_mutation(&mut book, is_bid, price, qty - prev_qty);
            }
            _ => panic!("Invalid input tuple: {:?}", tuple),
        }
        bbo_builder.update_builders(&mut book)
    }
    let result = bbo_builder.finish()?.into_struct("bbo").into_series();
    Ok(result)
}

#[instrument]
fn apply_simple_mutation<const N: usize>(
    book: &mut OrderBookWithTopNTracking<i64, i64, N>,
    is_bid: bool,
    price: i64,
    qty: i64,
) {
    if qty > 0 {
        debug!("Adding quantity");
        book.book_side(is_bid).add_qty(price, qty);
    } else {
        debug!("Deleting quantity");
        book.book_side(is_bid)
            .delete_qty(price, qty.abs())
            .expect("Invalid delete qty operation - likely deleted more than available qty");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_bbo_from_simple_mutations2() {
        let df = df! {
            "price" => [1i64, 2, 3, 4, 5, 9, 8, 7, 6],
            "qty" => [10i64, 20, 30, 40, 50, 90, 80, 70, 60],
            "is_bid" => [true, true, true, true, true, false, false, false, false],
        }
        .unwrap();

        let expected = df! {
            "price" => [1i64, 2, 3, 4, 5, 9, 8, 7, 6],
            "qty" => [10i64, 20, 30, 40, 50, 90, 80, 70, 60],
            "is_bid" => [true, true, true, true, true, false, false, false, false],
            "bid_price_1" => [1i64, 2, 3, 4, 5, 5, 5, 5, 5],
            "bid_price_2" => [None, Some(1i64), Some(2), Some(3), Some(4), Some(4), Some(4), Some(4), Some(4)],
            "bid_qty_1" => [10i64, 20, 30, 40, 50, 50, 50, 50, 50],
            "bid_qty_2" => [None, Some(10i64), Some(20), Some(30), Some(40), Some(40), Some(40), Some(40), Some(40)],
            "ask_price_1" => [None, None, None, None, None, Some(9i64), Some(8), Some(7), Some(6)],
            "ask_price_2" => [None, None, None, None, None, None, Some(9i64), Some(8), Some(7)],
            "ask_qty_1" => [None, None, None, None, None, Some(90i64), Some(80), Some(70), Some(60)],
            "ask_qty_2" => [None, None, None, None, None, None, Some(90i64), Some(80), Some(70)],
        }
        .unwrap();

        for level in 1..=2 {
            let bbo_struct = _pl_calculate_bbo(&df.get_columns(), level).unwrap();
            let df_with_bbo = df
                .clone()
                .with_column(bbo_struct)
                .expect("Failed to add BBO struct series to DataFrame")
                .unnest(["bbo"])
                .expect("Failed to unnest BBO struct series");

            if level == 1 {
                let expected_df = expected.clone().drop_many(&[
                    "bid_price_2",
                    "bid_qty_2",
                    "ask_price_2",
                    "ask_qty_2",
                ]);
                assert_eq!(df_with_bbo, expected_df);
            } else {
                assert_eq!(df_with_bbo, expected);
            }
        }
    }

    #[test]
    fn test_calculate_bbo_with_modifies() {
        let df = df! {
                "price" => [1i64, 2, 3, 4, 5, 9, 8, 7, 6, 1, 9],
                "qty" => [10i64, 20, 30, 40, 50, 90, 80, 70, 60, 1, 1],
                "is_bid" => [true, true, true, true, true, false, false, false, false, true, false],
                "prev_price" => [None, Some(1i64), Some(2), Some(3), Some(4), None, Some(9), Some(8), Some(7), Some(5), Some(6)],
                "prev_qty" => [None, Some(10i64), Some(20), Some(30), Some(40), None, Some(90), Some(80), Some(70), Some(50), Some(60)],
            }
            .unwrap();

        let expected = df! {
            "price" => [1i64, 2, 3, 4, 5, 9, 8, 7, 6, 1, 9],
            "qty" => [10i64, 20, 30, 40, 50, 90, 80, 70, 60, 1, 1],
            "is_bid" => [true, true, true, true, true, false, false, false, false, true, false],
            "prev_price" => [None, Some(1i64), Some(2), Some(3), Some(4), None, Some(9), Some(8), Some(7), Some(5), Some(6)],
            "prev_qty" => [None, Some(10i64), Some(20), Some(30), Some(40), None, Some(90), Some(80), Some(70), Some(50), Some(60)],
            "bid_price_1" => [1i64, 2, 3, 4, 5, 5, 5, 5, 5, 1, 1],
            "bid_price_2" => [Option::<i64>::None, None, None, None, None, None, None, None, None, None, None],            "bid_qty_1" => [10i64, 20, 30, 40, 50, 50, 50, 50, 50, 1, 1],
            "bid_qty_2" => [Option::<i64>::None, None, None, None, None, None, None, None, None, None, None],
            "ask_price_1" => [None, None, None, None, None, Some(9i64), Some(8), Some(7), Some(6), Some(6), Some(9)],
            "ask_price_2" => [Option::<i64>::None, None, None, None, None, None, None, None, None, None, None],
            "ask_qty_1" => [None, None, None, None, None, Some(90i64), Some(80), Some(70), Some(60), Some(60), Some(1)],
            "ask_qty_2" => [Option::<i64>::None, None, None, None, None, None, None, None, None, None, None],
            }
            .unwrap();

        for level in 1..=2 {
            let bbo_struct = _pl_calculate_bbo(&df.get_columns(), level).unwrap();
            let df_with_bbo = df
                .clone()
                .with_column(bbo_struct)
                .expect("Failed to add BBO struct series to DataFrame")
                .unnest(["bbo"])
                .expect("Failed to unnest BBO struct series");

            if level == 1 {
                let expected_df = expected.clone().drop_many(&[
                    "bid_price_2",
                    "bid_qty_2",
                    "ask_price_2",
                    "ask_qty_2",
                ]);
                assert_eq!(df_with_bbo, expected_df);
            } else {
                assert_eq!(df_with_bbo, expected);
            }
        }
    }

    #[test]
    fn test_calculate_bbo_with_modifies_cyclic() {
        let df = df! {
            "price" => vec![1i64, 6, 2,3,1, 5,4,6],
            "qty" => vec![1i64, 6, 2,3,1, 5,4,6],
            "is_bid" => vec![true, false, true, true, true, false, false, false],
            "prev_price" => vec![None, None, Some(1i64), Some(2), Some(3), Some(6), Some(5), Some(4)],
            "prev_qty" => vec![None, None, Some(1i64), Some(2), Some(3), Some(6), Some(5), Some(4)],
        }.unwrap();

        let expected = df! {
            "price" => vec![1, 6, 2,3,1, 5,4,6],
            "qty" => vec![1, 6, 2,3,1, 5,4,6],
            "is_bid" => vec![true, false, true, true, true, false, false, false],
            "prev_price" => vec![None, None, Some(1), Some(2), Some(3), Some(6), Some(5), Some(4)],
            "prev_qty" => vec![None, None, Some(1), Some(2), Some(3), Some(6), Some(5), Some(4)],
            "bid_price_1" => vec![1, 1, 2, 3, 1, 1, 1, 1],
            "bid_price_2" => vec![Option::<i64>::None, None, None, None, None, None, None, None],
            "bid_qty_1" => vec![1, 1, 2, 3, 1, 1, 1, 1],
            "bid_qty_2" => vec![Option::<i64>::None, None, None, None, None, None, None, None],
            "ask_price_1" => vec![None, Some(6), Some(6), Some(6), Some(6), Some(5), Some(4), Some(6)],
            "ask_price_2" => vec![Option::<i64>::None, None, None, None, None, None, None, None],
            "ask_qty_1" => vec![None, Some(6), Some(6), Some(6), Some(6), Some(5), Some(4), Some(6)],
            "ask_qty_2" => vec![Option::<i64>::None, None, None, None, None, None, None, None],
        }.unwrap();

        for level in 1..=2 {
            let bbo_struct = _pl_calculate_bbo(&df.get_columns(), level).unwrap();
            let df_with_bbo = df
                .clone()
                .with_column(bbo_struct)
                .expect("Failed to add BBO struct series to DataFrame")
                .unnest(["bbo"])
                .expect("Failed to unnest BBO struct series");

            if level == 1 {
                let expected_df = expected.clone().drop_many(&[
                    "bid_price_2",
                    "bid_qty_2",
                    "ask_price_2",
                    "ask_qty_2",
                ]);
                assert_eq!(df_with_bbo, expected_df);
            } else {
                assert_eq!(df_with_bbo, expected);
            }
        }
    }
}
