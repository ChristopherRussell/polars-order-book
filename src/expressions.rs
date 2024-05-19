#![allow(clippy::unused_unit)]
use polars::prelude::*;
use pyo3_polars::derive::polars_expr;

use crate::{book_side::BookSide, order_book::OrderBook};
use itertools::izip;
use polars::datatypes::BooleanType;

fn bbo_struct(input_fields: &[Field]) -> PolarsResult<Field> {
    let price_field = &input_fields[0];
    let qty_field = &input_fields[1];

    let bbo_struct = DataType::Struct(vec![
        Field::new("best_bid", price_field.data_type().clone()),
        Field::new("best_bid_qty", qty_field.data_type().clone()),
        Field::new("best_ask", price_field.data_type().clone()),
        Field::new("best_ask_qty", qty_field.data_type().clone()),
    ]);

    Ok(Field::new("shifted", bbo_struct))
}

#[polars_expr(output_type_func=bbo_struct)]
pub fn pl_calculate_bbo(inputs: &[Series]) -> PolarsResult<Series> {
    _pl_calculate_bbo(inputs)
}

fn _pl_calculate_bbo(inputs: &[Series]) -> PolarsResult<Series> {
    if inputs.len() < 3 {
        panic!("Expected 3 input columns, got: {}", inputs.len());
    }
    let price = inputs[0].i64()?;
    let qty = inputs[1].i64()?;
    let is_bid = inputs[2].bool()?;
    calculate_bbo(price, qty, is_bid)
}

fn calculate_bbo(
    price_array: &ChunkedArray<Int64Type>,
    qty_array: &ChunkedArray<Int64Type>,
    is_bid_array: &ChunkedArray<BooleanType>,
) -> PolarsResult<Series> {
    let length = price_array.len();
    let mut best_bid_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("best_bid", length);
    let mut best_bid_qty_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("best_bid_qty", length);
    let mut best_ask_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("best_ask", length);
    let mut best_ask_qty_builder: PrimitiveChunkedBuilder<Int64Type> =
        PrimitiveChunkedBuilder::new("best_ask_qty", length);

    let mut book: OrderBook<i64, i64> = OrderBook::default();

    for tuple in izip!(
        is_bid_array.into_iter(),
        price_array.into_iter(),
        qty_array.into_iter()
    ) {
        if let (Some(is_bid), Some(price), Some(qty)) = tuple {
            if qty > 0 {
                book.book_side(is_bid).add_qty(price, qty)
            } else {
                book.book_side(is_bid)
                    .delete_qty(price, qty.abs())
                    .expect("Invalid delete qty operation - likely deleted more than available qty")
            }

            update_builders(
                book.book_side(true),
                &mut best_bid_builder,
                &mut best_bid_qty_builder,
            );

            update_builders(
                book.book_side(false),
                &mut best_ask_builder,
                &mut best_ask_qty_builder,
            );
        } else {
            panic!("Invalid input tuple: {:?}", tuple);
        }
    }
    let result = df!(
        "best_bid"=>best_bid_builder.finish().into_series(),
        "best_bid_qty"=>best_bid_qty_builder.finish().into_series(),
        "best_ask"=>best_ask_builder.finish().into_series(),
        "best_ask_qty"=>best_ask_qty_builder.finish().into_series()
    )?
    .into_struct("bbo")
    .into_series();
    Ok(result)
}

fn update_builders(
    book_side: &BookSide<i64, i64>,
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
    fn test_calculate_bbo() {
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
            "best_bid" => [1i64, 2, 3, 4, 5, 5, 5, 5, 5],
            "best_bid_qty" => [10i64, 20, 30, 40, 50, 50, 50, 50, 50],
            "best_ask" => [None, None, None, None, None, Some(9i64), Some(8), Some(7), Some(6)],
            "best_ask_qty" => [None, None, None, None, None, Some(90i64), Some(80), Some(70), Some(60)],
        }
        .unwrap();
        assert_eq!(df, expected);
    }
}
