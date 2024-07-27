#![allow(clippy::unused_unit)]

use itertools::izip;
use polars::datatypes::BooleanType;
use polars::prelude::*;
use pyo3_polars::derive::polars_expr;

use order_book::{
    book_side_tracked_basic::BookSideWithBasicTracking,
    order_book_tracked_basic::OrderBookWithBasicTracking,
};

fn bbo_struct(input_fields: &[Field]) -> PolarsResult<Field> {
    let price_field = &input_fields[0];
    let qty_field = &input_fields[1];

    let bbo_struct = DataType::Struct(vec![
        Field::new("bid_price_1", price_field.data_type().clone()),
        Field::new("bid_qty_1", qty_field.data_type().clone()),
        Field::new("ask_price_1", price_field.data_type().clone()),
        Field::new("ask_qty_1", qty_field.data_type().clone()),
    ]);
    Ok(Field::new("bbo", bbo_struct))
}

#[polars_expr(output_type_func = bbo_struct)]
pub fn pl_calculate_bbo_basic(inputs: &[Series]) -> PolarsResult<Series> {
    _pl_calculate_bbo(inputs)
}

fn _pl_calculate_bbo(inputs: &[Series]) -> PolarsResult<Series> {
    match inputs.len() {
        3 | 5 => {}
        _ => {
            let input_names = inputs
                .iter()
                .map(|s| s.name())
                .collect::<Vec<&str>>()
                .join(", ");
            panic!("Expected 3 or 5 input columns: price, qty, is_bid, (prev_price, prev_qty) but got {} columns called:\n    {}", inputs.len(), input_names)
        }
    }

    let price = inputs[0].i64()?;
    let qty = inputs[1].i64()?;
    let is_bid = inputs[2].bool()?;
    let prev_price = inputs.get(3);
    let prev_qty = inputs.get(4);

    match (prev_price, prev_qty) {
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
    }
}

/// Calculate the best bid and best ask prices and quantities
/// using price-point add and delete mutations.
pub fn calculate_bbo_from_simple_mutations_basic_tracking(
    price_array: &ChunkedArray<Int64Type>,
    qty_array: &ChunkedArray<Int64Type>,
    is_bid_array: &ChunkedArray<BooleanType>,
) -> PolarsResult<Series> {
    let length = price_array.len();
    let mut bid_price_1_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("bid_price_1", length);
    let mut bid_qty_1_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("bid_qty_1", length);
    let mut ask_price_1_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("ask_price_1", length);
    let mut ask_qty_1_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("ask_qty_1", length);

    let mut book: OrderBookWithBasicTracking<i64, i64> = OrderBookWithBasicTracking::default();
    for tuple in izip!(
        is_bid_array.into_iter(),
        price_array.into_iter(),
        qty_array.into_iter()
    ) {
        if let (Some(is_bid), Some(price), Some(qty)) = tuple {
            apply_simple_mutation(&mut book, is_bid, price, qty);

            update_builders_one_side(
                book.book_side(true),
                &mut bid_price_1_builder,
                &mut bid_qty_1_builder,
            );

            update_builders_one_side(
                book.book_side(false),
                &mut ask_price_1_builder,
                &mut ask_qty_1_builder,
            );
        } else {
            panic!("Invalid input tuple: {:?}", tuple);
        }
    }
    let result = df!(
        "bid_price_1"=>bid_price_1_builder.finish().into_series(),
        "bid_qty_1"=>bid_qty_1_builder.finish().into_series(),
        "ask_price_1"=>ask_price_1_builder.finish().into_series(),
        "ask_qty_1"=>ask_qty_1_builder.finish().into_series()
    )?
    .into_struct("bbo")
    .into_series();
    Ok(result)
}

/// Calculate the best bid and best ask prices and quantities
/// using price-point mutations which may include modifies, i.e.
/// a delete and an add operation in a single row.
pub fn calculate_bbo_with_modifies_basic_tracking(
    price_array: &ChunkedArray<Int64Type>,
    qty_array: &ChunkedArray<Int64Type>,
    is_bid_array: &ChunkedArray<BooleanType>,
    prev_price_array: &ChunkedArray<Int64Type>,
    prev_qty_array: &ChunkedArray<Int64Type>,
) -> PolarsResult<Series> {
    let length = price_array.len();
    let mut bid_price_1_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("bid_price_1", length);
    let mut bid_qty_1_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("bid_qty_1", length);
    let mut ask_price_1_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("ask_price_1", length);
    let mut ask_qty_1_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("ask_qty_1", length);

    let mut book: OrderBookWithBasicTracking<i64, i64> = OrderBookWithBasicTracking::default();
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
        update_builders_one_side(
            book.book_side(true),
            &mut bid_price_1_builder,
            &mut bid_qty_1_builder,
        );

        update_builders_one_side(
            book.book_side(false),
            &mut ask_price_1_builder,
            &mut ask_qty_1_builder,
        );
    }
    let result = df!(
        "bid_price_1"=>bid_price_1_builder.finish().into_series(),
        "bid_qty_1"=>bid_qty_1_builder.finish().into_series(),
        "ask_price_1"=>ask_price_1_builder.finish().into_series(),
        "ask_qty_1"=>ask_qty_1_builder.finish().into_series()
    )?
    .into_struct("bbo")
    .into_series();
    Ok(result)
}

fn apply_simple_mutation(
    book: &mut OrderBookWithBasicTracking<i64, i64>,
    is_bid: bool,
    price: i64,
    qty: i64,
) {
    if qty > 0 {
        book.book_side(is_bid).add_qty(price, qty)
    } else {
        book.book_side(is_bid)
            .delete_qty(price, qty.abs())
            .expect("Invalid delete qty operation - likely deleted more than available qty")
    }
}

fn update_builders_one_side(
    book_side: &BookSideWithBasicTracking<i64, i64>,
    price_builder: &mut PrimitiveChunkedBuilder<Int64Type>,
    qty_builder: &mut PrimitiveChunkedBuilder<Int64Type>,
) {
    price_builder.append_option(book_side.best_price);
    qty_builder.append_option(book_side.best_price_qty);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_bbo_from_simple_mutations() {
        let mut df = df! {
            "price" => [1i64, 2, 3, 4, 5, 9, 8, 7, 6],
            "qty" => [10i64, 20, 30, 40, 50, 90, 80, 70, 60],
            "is_bid" => [true, true, true, true, true, false, false, false, false],
        }
        .unwrap();
        let inputs = df.get_columns();

        let bbo_struct = _pl_calculate_bbo(inputs).unwrap();
        df = df
            .with_column(bbo_struct)
            .expect("Failed to add BBO struct series to DataFrame")
            .unnest(["bbo"])
            .expect("Failed to unnest BBO struct series");

        let expected = df! {
            "price" => [1i64, 2, 3, 4, 5, 9, 8, 7, 6],
            "qty" => [10i64, 20, 30, 40, 50, 90, 80, 70, 60],
            "is_bid" => [true, true, true, true, true, false, false, false, false],
            "bid_price_1" => [1i64, 2, 3, 4, 5, 5, 5, 5, 5],
            "bid_qty_1" => [10i64, 20, 30, 40, 50, 50, 50, 50, 50],
            "ask_price_1" => [None, None, None, None, None, Some(9i64), Some(8), Some(7), Some(6)],
            "ask_qty_1" => [None, None, None, None, None, Some(90i64), Some(80), Some(70), Some(60)],
        }.unwrap();
        assert_eq!(df, expected);
    }

    #[test]
    fn test_calculate_bbo_with_modifies() {
        let mut df = df! {
            "price" => [1i64, 2, 3, 4, 5, 9, 8, 7, 6, 1, 9],
            "qty" => [10i64, 20, 30, 40, 50, 90, 80, 70, 60, 1, 1],
            "is_bid" => [true, true, true, true, true, false, false, false, false, true, false],
            "prev_price" => [None, Some(1i64), Some(2), Some(3), Some(4), None, Some(9), Some(8), Some(7), Some(5), Some(6)],
            "prev_qty" => [None, Some(10i64), Some(20), Some(30), Some(40), None, Some(90), Some(80), Some(70), Some(50), Some(60)],
        }
            .unwrap();
        let inputs = df.get_columns();

        let bbo_struct = _pl_calculate_bbo(inputs).unwrap();
        df = df
            .with_column(bbo_struct)
            .expect("Failed to add BBO struct series to DataFrame")
            .unnest(["bbo"])
            .expect("Failed to unnest BBO struct series");
        let expected = df! {
            "price" => [1i64, 2, 3, 4, 5, 9, 8, 7, 6, 1, 9],
            "qty" => [10i64, 20, 30, 40, 50, 90, 80, 70, 60, 1, 1],
            "is_bid" => [true, true, true, true, true, false, false, false, false, true, false],
            "prev_price" => [None, Some(1i64), Some(2), Some(3), Some(4), None, Some(9), Some(8), Some(7), Some(5), Some(6)],
            "prev_qty" => [None, Some(10i64), Some(20), Some(30), Some(40), None, Some(90), Some(80), Some(70), Some(50), Some(60)],
            "bid_price_1" => [1i64, 2, 3, 4, 5, 5, 5, 5, 5, 1, 1],
            "bid_qty_1" => [10i64, 20, 30, 40, 50, 50, 50, 50, 50, 1, 1],
            "ask_price_1" => [None, None, None, None, None, Some(9i64), Some(8), Some(7), Some(6), Some(6), Some(9)],
            "ask_qty_1" => [None, None, None, None, None, Some(90i64), Some(80), Some(70), Some(60), Some(60), Some(1)],
        }
            .unwrap();
        assert_eq!(df, expected);
    }

    #[test]
    fn test_calculate_bbo_with_modifies_cyclic() {
        let mut df = df! {
            "price" => vec![1i64, 6, 2,3,1, 5,4,6],
            "qty" => vec![1i64, 6, 2,3,1, 5,4,6],
            "is_bid" => vec![true, false, true, true, true, false, false, false],
            "prev_price" => vec![None, None, Some(1i64), Some(2), Some(3), Some(6), Some(5), Some(4)],
            "prev_qty" => vec![None, None, Some(1i64), Some(2), Some(3), Some(6), Some(5), Some(4)],
        }.unwrap();

        let inputs = df.get_columns();

        let bbo_struct = _pl_calculate_bbo(inputs).unwrap();
        let df = df
            .with_column(bbo_struct)
            .expect("Failed to add BBO struct series to DataFrame")
            .unnest(["bbo"])
            .expect("Failed to unnest BBO struct series");

        let expected_values = df! {
            "price" => vec![1, 6, 2,3,1, 5,4,6],
            "qty" => vec![1, 6, 2,3,1, 5,4,6],
            "is_bid" => vec![true, false, true, true, true, false, false, false],
            "prev_price" => vec![None, None, Some(1), Some(2), Some(3), Some(6), Some(5), Some(4)],
            "prev_qty" => vec![None, None, Some(1), Some(2), Some(3), Some(6), Some(5), Some(4)],
            "bid_price_1" => vec![1, 1, 2, 3, 1, 1, 1, 1],
            "bid_qty_1" => vec![1, 1, 2, 3, 1, 1, 1, 1],
            "ask_price_1" => vec![None, Some(6), Some(6), Some(6), Some(6), Some(5), Some(4), Some(6)],
            "ask_qty_1" => vec![None, Some(6), Some(6), Some(6), Some(6), Some(5), Some(4), Some(6)],
        }.unwrap();

        assert_eq!(df, expected_values);
    }
}
