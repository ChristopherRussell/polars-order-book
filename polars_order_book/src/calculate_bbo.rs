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
    } else {
        let bbo_struct = DataType::Struct(vec![
            Field::new("bid_price_1", price_field.data_type().clone()),
            Field::new("bid_qty_1", qty_field.data_type().clone()),
            Field::new("ask_price_1", price_field.data_type().clone()),
            Field::new("ask_qty_1", qty_field.data_type().clone()),
        ]);
        Ok(Field::new("bbo", bbo_struct))
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

    Ok(builder.finish()?.into_struct("bbo").into_series())
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

    Ok(builder.finish()?.into_struct("bbo").into_series())
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
    let updates_iter = crate::update::PriceUpdateIterator::new(
        inputs[2].bool()?.into_iter(),
        inputs[0].i64()?.into_iter(),
        inputs[1].i64()?.into_iter(),
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
    let updates_iter = crate::update::PriceMutationIterator::new(
        inputs[2].bool()?.into_iter(),
        inputs[0].i64()?.into_iter(),
        inputs[1].i64()?.into_iter(),
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
    if inputs.len() != 5 {
        return Err(PolarsError::ShapeMismatch(
            "Expected 5 input columns: price, qty, is_bid, prev_price, prev_qty".into(),
        ));
    }

    let updates_iter = crate::update::PriceMutationWithModifyIterator::new(
        inputs[2].bool()?.into_iter(),
        inputs[0].i64()?.into_iter(),
        inputs[1].i64()?.into_iter(),
        inputs[3].i64()?.into_iter(),
        inputs[4].i64()?.into_iter(),
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
