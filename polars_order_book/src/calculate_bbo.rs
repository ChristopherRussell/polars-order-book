#![allow(clippy::unused_unit)]

use crate::{
    errors::PolarsOrderBookError,
    output::{
        OutputBuilder, TopNLevelsDataframeBuilder, TopNLevelsOutput, TopOfBookDataframeBuilder,
        TopOfBookOutput,
    },
    update::{
        ApplyUpdate, PriceMutation, PriceMutationWithModify, PriceUpdate, UpdateMissingValueError,
    },
};
use order_book::order_book_tracked::OrderBookWithTopNTracking;
use order_book::order_book_tracked_basic::OrderBookWithBasicTracking;
use order_book_core::order_book::BidAskBook;
use polars::prelude::*;
use pyo3_polars::derive::polars_expr;
use serde::Deserialize;

#[derive(Deserialize)]
struct TopNLevelsKwargs {
    n: usize,
}

fn bbo_struct(input_fields: &[Field], kwargs: TopNLevelsKwargs) -> PolarsResult<Field> {
    let price_field = &input_fields[0];
    let qty_field = &input_fields[1];
    let n = kwargs.n;

    if n > 1 {
        let mut bbo_struct = vec![];
        for i in 1..=n {
            bbo_struct.push(Field::new(
                PlSmallStr::from_str(&format!("bid_price_{}", i)),
                price_field.dtype().clone(),
            ));
            bbo_struct.push(Field::new(
                PlSmallStr::from_str(&format!("bid_qty_{}", i)),
                qty_field.dtype().clone(),
            ));
            bbo_struct.push(Field::new(
                PlSmallStr::from_str(&format!("ask_price_{}", i)),
                price_field.dtype().clone(),
            ));
            bbo_struct.push(Field::new(
                PlSmallStr::from_str(&format!("ask_qty_{}", i)),
                qty_field.dtype().clone(),
            ));
        }
        Ok(Field::new(
            PlSmallStr::from_str("bbo"),
            DataType::Struct(bbo_struct),
        ))
    } else {
        let bbo_struct = DataType::Struct(vec![
            Field::new(
                PlSmallStr::from_str("bid_price_1"),
                price_field.dtype().clone(),
            ),
            Field::new(PlSmallStr::from_str("bid_qty_1"), qty_field.dtype().clone()),
            Field::new(
                PlSmallStr::from_str("ask_price_1"),
                price_field.dtype().clone(),
            ),
            Field::new(PlSmallStr::from_str("ask_qty_1"), qty_field.dtype().clone()),
        ]);
        Ok(Field::new(PlSmallStr::from_str("bbo"), bbo_struct))
    }
}

fn calculate_bbo_top_of_book<U, I>(inputs: &[Series], updates_iter: I) -> PolarsResult<Series>
where
    U: ApplyUpdate<i64, i64, OrderBookWithBasicTracking<i64, i64>>,
    I: Iterator<Item = Result<U, UpdateMissingValueError>>,
{
    let mut builder = TopOfBookDataframeBuilder::new(inputs[0].len());
    let mut book = OrderBookWithBasicTracking::<i64, i64>::default();

    for update in updates_iter {
        update
            .map_err(PolarsOrderBookError::from)?
            .apply_update(&mut book)
            .map_err(PolarsOrderBookError::from)?;

        let output = TopOfBookOutput {
            bid_price_1: book.bids().best_price.map(|px| px.0),
            bid_qty_1: book.bids().best_price_qty,
            ask_price_1: book.asks().best_price.map(|px| px.0),
            ask_qty_1: book.asks().best_price_qty,
        };
        builder.append(output);
    }

    Ok(builder
        .finish()?
        .into_struct(PlSmallStr::from_str("bbo"))
        .into_series())
}

fn calculate_bbo_top_n_levels<U, I, const N: usize>(
    inputs: &[Series],
    updates_iter: I,
) -> PolarsResult<Series>
where
    U: ApplyUpdate<i64, i64, OrderBookWithTopNTracking<i64, i64, N>>,
    I: Iterator<Item = Result<U, UpdateMissingValueError>>,
{
    let mut builder = TopNLevelsDataframeBuilder::<N>::new(inputs[0].len());
    let mut book = OrderBookWithTopNTracking::<i64, i64, N>::default();

    for update in updates_iter {
        update
            .map_err(PolarsOrderBookError::from)?
            .apply_update(&mut book)
            .map_err(PolarsOrderBookError::from)?;

        let output = TopNLevelsOutput {
            bid_levels: book.bids.top_n(),
            ask_levels: book.asks.top_n(),
        };
        builder.append(output);
    }

    Ok(builder
        .finish()?
        .into_struct(PlSmallStr::from_str("bbo"))
        .into_series())
}

macro_rules! generate_n_cases {
    ($func:ident, $inputs:expr, $updates_iter:expr, $kwargs:expr, $($n:expr),+) => {
        match $kwargs.n {
            1 => calculate_bbo_top_of_book::<$func<i64, i64>, _>($inputs, $updates_iter),
            $( $n => calculate_bbo_top_n_levels::<$func<i64, i64>, _, $n>($inputs, $updates_iter), )+
            _ => Err(PolarsError::ComputeError(
                format!("Unsupported number of levels: {}", $kwargs.n).into(),
            )),
        }
    };
}

#[polars_expr(output_type_func_with_kwargs = bbo_struct)]
pub fn pl_calculate_bbo_price_update(
    inputs: &[Series],
    kwargs: TopNLevelsKwargs,
) -> PolarsResult<Series> {
    _pl_calculate_bbo_price_update(inputs, kwargs)
}

fn _pl_calculate_bbo_price_update(
    inputs: &[Series],
    kwargs: TopNLevelsKwargs,
) -> PolarsResult<Series> {
    let updates_iter = crate::update::PriceUpdateIterator::new(
        inputs[2].bool()?.into_iter(), // is_bid
        inputs[0].i64()?.into_iter(),  // price
        inputs[1].i64()?.into_iter(),  // qty
    );

    generate_n_cases!(
        PriceUpdate,
        inputs,
        updates_iter,
        kwargs,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15,
        16,
        17,
        18,
        19,
        20
    )
}

#[polars_expr(output_type_func_with_kwargs = bbo_struct)]
pub fn pl_calculate_bbo_price_mutation(
    inputs: &[Series],
    kwargs: TopNLevelsKwargs,
) -> PolarsResult<Series> {
    _pl_calculate_bbo_price_mutation(inputs, kwargs)
}

fn _pl_calculate_bbo_price_mutation(
    inputs: &[Series],
    kwargs: TopNLevelsKwargs,
) -> PolarsResult<Series> {
    let updates_iter = crate::update::PriceMutationIterator::new(
        inputs[2].bool()?.into_iter(), // is_bid
        inputs[0].i64()?.into_iter(),  // price
        inputs[1].i64()?.into_iter(),  // qty
    );

    generate_n_cases!(
        PriceMutation,
        inputs,
        updates_iter,
        kwargs,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15,
        16,
        17,
        18,
        19,
        20
    )
}

#[polars_expr(output_type_func_with_kwargs = bbo_struct)]
pub fn pl_calculate_bbo_mutation_modify(
    inputs: &[Series],
    kwargs: TopNLevelsKwargs,
) -> PolarsResult<Series> {
    _pl_calculate_bbo_mutation_modify(inputs, kwargs)
}

fn _pl_calculate_bbo_mutation_modify(
    inputs: &[Series],
    kwargs: TopNLevelsKwargs,
) -> PolarsResult<Series> {
    if inputs.len() != 5 {
        return Err(PolarsError::ShapeMismatch(
            "Expected 5 input columns: price, qty, is_bid, prev_price, prev_qty".into(),
        ));
    }

    let updates_iter = crate::update::PriceMutationWithModifyIterator::new(
        inputs[2].bool()?.into_iter(), // is_bid
        inputs[0].i64()?.into_iter(),  // price
        inputs[1].i64()?.into_iter(),  // qty
        inputs[3].i64()?.into_iter(),  // prev_price
        inputs[4].i64()?.into_iter(),  // prev_qty
    );

    generate_n_cases!(
        PriceMutationWithModify,
        inputs,
        updates_iter,
        kwargs,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15,
        16,
        17,
        18,
        19,
        20
    )
}

mod tests {
    #[allow(unused_imports)] // Not sure why clippy is complaining about unused imports here
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
        let inputs = inputs
            .iter()
            .map(|c| c.clone().take_materialized_series())
            .collect::<Vec<_>>();
        let kwargs = TopNLevelsKwargs { n: 1 };

        let bbo_struct = _pl_calculate_bbo_price_mutation(&inputs, kwargs).unwrap();
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
        let inputs = inputs
            .iter()
            .map(|c| c.clone().take_materialized_series())
            .collect::<Vec<_>>();
        let kwargs = TopNLevelsKwargs { n: 1 };
        let bbo_struct = _pl_calculate_bbo_mutation_modify(&inputs, kwargs).unwrap();
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
        let inputs = inputs
            .iter()
            .map(|c| c.clone().take_materialized_series())
            .collect::<Vec<_>>();
        let kwargs = TopNLevelsKwargs { n: 1 };

        let bbo_struct = _pl_calculate_bbo_mutation_modify(&inputs, kwargs).unwrap();
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
            let kwargs = TopNLevelsKwargs { n: level };
            let inputs = df
                .get_columns()
                .iter()
                .map(|c| c.clone().take_materialized_series())
                .collect::<Vec<_>>();
            let bbo_struct = _pl_calculate_bbo_price_mutation(&inputs, kwargs).unwrap();
            let df_with_bbo = df
                .clone()
                .with_column(bbo_struct)
                .expect("Failed to add BBO struct series to DataFrame")
                .unnest(["bbo"])
                .expect("Failed to unnest BBO struct series");

            if level == 1 {
                let expected_df = expected.clone().drop_many([
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
    fn test_calculate_bbo_with_modifies2() {
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
            let kwargs = TopNLevelsKwargs { n: level };
            let inputs = df
                .get_columns()
                .iter()
                .map(|c| c.clone().take_materialized_series())
                .collect::<Vec<_>>();
            let bbo_struct = _pl_calculate_bbo_mutation_modify(&inputs, kwargs).unwrap();
            let df_with_bbo = df
                .clone()
                .with_column(bbo_struct)
                .expect("Failed to add BBO struct series to DataFrame")
                .unnest(["bbo"])
                .expect("Failed to unnest BBO struct series");

            if level == 1 {
                let expected_df = expected.clone().drop_many([
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
    fn test_calculate_bbo_with_modifies_cyclic2() {
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
            let kwargs = TopNLevelsKwargs { n: level };
            let inputs = df
                .get_columns()
                .iter()
                .map(|c| c.clone().take_materialized_series())
                .collect::<Vec<_>>();
            let bbo_struct = _pl_calculate_bbo_mutation_modify(&inputs, kwargs).unwrap();
            let df_with_bbo = df
                .clone()
                .with_column(bbo_struct)
                .expect("Failed to add BBO struct series to DataFrame")
                .unnest(["bbo"])
                .expect("Failed to unnest BBO struct series");

            if level == 1 {
                let expected_df = expected.clone().drop_many([
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
