#![allow(clippy::unused_unit)]

use itertools::izip;
use polars::datatypes::BooleanType;
use polars::prelude::*;
use pyo3_polars::derive::polars_expr;

use order_book::{book_side::BookSide, order_book::OrderBook};

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

#[polars_expr(output_type_func = bbo_struct)]
pub fn pl_calculate_bbo(inputs: &[Series]) -> PolarsResult<Series> {
    _pl_calculate_bbo(inputs)
}

fn _pl_calculate_bbo(inputs: &[Series]) -> PolarsResult<Series> {
    match inputs.len() {
        3 | 5 => {}
        _ => {
            let input_names = inputs.iter().map(|s| s.name()).collect::<Vec<&str>>().join(", ");
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
            calculate_bbo_with_modifies(price, qty, is_bid, prev_price_chunked, prev_qty_chunked)
        }
        (None, None) => calculate_bbo_from_simple_mutations(price, qty, is_bid),
        _ => panic!("Expected both prev_price and prev_qty or neither, got: {:?} and {:?}", prev_price, prev_qty)
    }
}

/// Calculate the best bid and best ask prices and quantities
/// using price-point add and delete mutations.
fn calculate_bbo_from_simple_mutations(
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
            apply_simple_mutation(&mut book, is_bid, price, qty);

            update_builders_one_side(
                book.book_side(true),
                &mut best_bid_builder,
                &mut best_bid_qty_builder,
            );

            update_builders_one_side(
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

/// Calculate the best bid and best ask prices and quantities
/// using price-point mutations which may include modifies, i.e.
/// a delete and an add operation in a single row.
fn calculate_bbo_with_modifies(
    price_array: &ChunkedArray<Int64Type>,
    qty_array: &ChunkedArray<Int64Type>,
    is_bid_array: &ChunkedArray<BooleanType>,
    prev_price_array: &ChunkedArray<Int64Type>,
    prev_qty_array: &ChunkedArray<Int64Type>,
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
        qty_array.into_iter(),
        prev_price_array.into_iter(),
        prev_qty_array.into_iter()
    ) {
        match tuple {
            (Some(is_bid), Some(price), Some(qty), None, None) => {
                println!("simple: is_bid: {}, price: {}, qty: {}", is_bid, price, qty);
                apply_simple_mutation(&mut book, is_bid, price, qty);
            }
            (Some(is_bid), Some(price), Some(qty), Some(prev_price), Some(prev_qty)) => {
                println!("mod: is_bid: {}, price: {}, qty: {}, prev_price: {}, prev_qty: {}", is_bid, price, qty, prev_price, prev_qty);
                book.modify_qty(is_bid, prev_price, prev_qty, price, qty)
            }
            (Some(is_bid), Some(price), Some(qty), None, Some(prev_qty)) => {
                println!("simple mod: is_bid: {}, price: {}, qty: {}, prev_qty: {}", is_bid, price, qty, prev_qty);
                apply_simple_mutation(&mut book, is_bid, price, qty - prev_qty);
            }
            _ => panic!("Invalid input tuple: {:?}", tuple),
        }
        update_builders_one_side(
            book.book_side(true),
            &mut best_bid_builder,
            &mut best_bid_qty_builder,
        );

        update_builders_one_side(
            book.book_side(false),
            &mut best_ask_builder,
            &mut best_ask_qty_builder,
        );
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

fn apply_simple_mutation(book: &mut OrderBook<i64, i64>, is_bid: bool, price: i64, qty: i64) {
    if qty > 0 {
        book.book_side(is_bid).add_qty(price, qty)
    } else {
        book.book_side(is_bid)
            .delete_qty(price, qty.abs())
            .expect("Invalid delete qty operation - likely deleted more than available qty")
    }
}

fn update_builders_one_side(
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
            "best_bid" => [1i64, 2, 3, 4, 5, 5, 5, 5, 5],
            "best_bid_qty" => [10i64, 20, 30, 40, 50, 50, 50, 50, 50],
            "best_ask" => [None, None, None, None, None, Some(9i64), Some(8), Some(7), Some(6)],
            "best_ask_qty" => [None, None, None, None, None, Some(90i64), Some(80), Some(70), Some(60)],
        }
            .unwrap();
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
            "best_bid" => [1i64, 2, 3, 4, 5, 5, 5, 5, 5, 1, 1],
            "best_bid_qty" => [10i64, 20, 30, 40, 50, 50, 50, 50, 50, 1, 1],
            "best_ask" => [None, None, None, None, None, Some(9i64), Some(8), Some(7), Some(6), Some(6), Some(9)],
            "best_ask_qty" => [None, None, None, None, None, Some(90i64), Some(80), Some(70), Some(60), Some(60), Some(1)],
        }
            .unwrap();
        assert_eq!(df, expected);
    }
}
