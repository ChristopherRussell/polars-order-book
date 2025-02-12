use order_book_core::price_level::{AskPrice, BidPrice, PriceLevel};
use polars::{
    df,
    error::PolarsResult,
    frame::DataFrame,
    prelude::{ChunkedBuilder, Int64Type, IntoColumn, PlSmallStr, PrimitiveChunkedBuilder},
    series::IntoSeries,
};

pub trait OutputBuilder<T> {
    fn append(&mut self, output: T);
    fn finish(self) -> PolarsResult<DataFrame>;
}

pub struct TopOfBookOutput {
    pub bid_price_1: Option<i64>,
    pub bid_qty_1: Option<i64>,
    pub ask_price_1: Option<i64>,
    pub ask_qty_1: Option<i64>,
}

pub struct TopOfBookDataframeBuilder {
    bid_price_1: PrimitiveChunkedBuilder<Int64Type>,
    bid_qty_1: PrimitiveChunkedBuilder<Int64Type>,
    ask_price_1: PrimitiveChunkedBuilder<Int64Type>,
    ask_qty_1: PrimitiveChunkedBuilder<Int64Type>,
}

impl OutputBuilder<TopOfBookOutput> for TopOfBookDataframeBuilder {
    fn append(&mut self, output: TopOfBookOutput) {
        self.bid_price_1.append_option(output.bid_price_1);
        self.bid_qty_1.append_option(output.bid_qty_1);
        self.ask_price_1.append_option(output.ask_price_1);
        self.ask_qty_1.append_option(output.ask_qty_1);
    }
    fn finish(self) -> PolarsResult<DataFrame> {
        df!(
            "bid_price_1"=>self.bid_price_1.finish().into_series()  ,
            "bid_qty_1"=>self.bid_qty_1.finish().into_series(),
            "ask_price_1"=>self.ask_price_1.finish().into_series(),
            "ask_qty_1"=>self.ask_qty_1.finish().into_series(),
        )
    }
}

impl TopOfBookDataframeBuilder {
    pub fn new(capacity: usize) -> Self {
        Self {
            bid_price_1: PrimitiveChunkedBuilder::new(PlSmallStr::from("bid_price_1"), capacity),
            bid_qty_1: PrimitiveChunkedBuilder::new(PlSmallStr::from("bid_qty_1"), capacity),
            ask_price_1: PrimitiveChunkedBuilder::new(PlSmallStr::from("ask_price_1"), capacity),
            ask_qty_1: PrimitiveChunkedBuilder::new(PlSmallStr::from("ask_qty_1"), capacity),
        }
    }
}

pub struct TopNLevelsOutput<'a, const N: usize> {
    pub bid_levels: &'a [Option<PriceLevel<BidPrice<i64>, i64>>; N],
    pub ask_levels: &'a [Option<PriceLevel<AskPrice<i64>, i64>>; N],
}

pub struct TopNLevelsDataframeBuilder<const N: usize> {
    bid_prices: [PrimitiveChunkedBuilder<Int64Type>; N],
    bid_qtys: [PrimitiveChunkedBuilder<Int64Type>; N],
    ask_prices: [PrimitiveChunkedBuilder<Int64Type>; N],
    ask_qtys: [PrimitiveChunkedBuilder<Int64Type>; N],
}

impl<'a, const N: usize> OutputBuilder<TopNLevelsOutput<'a, N>> for TopNLevelsDataframeBuilder<N> {
    fn append(&mut self, output: TopNLevelsOutput<'a, N>) {
        for i in 0..N {
            if let Some(level) = output.bid_levels[i] {
                self.bid_prices[i].append_value(level.price.0);
                self.bid_qtys[i].append_value(level.qty);
            } else {
                self.bid_prices[i].append_null();
                self.bid_qtys[i].append_null();
            }
            if let Some(level) = output.ask_levels[i] {
                self.ask_prices[i].append_value(level.price.0);
                self.ask_qtys[i].append_value(level.qty);
            } else {
                self.ask_prices[i].append_null();
                self.ask_qtys[i].append_null();
            }
        }
    }

    fn finish(self) -> PolarsResult<DataFrame> {
        let columns = self
            .bid_prices
            .into_iter()
            .chain(self.bid_qtys)
            .chain(self.ask_prices)
            .chain(self.ask_qtys)
            .map(|builder| builder.finish().into_column())
            .collect();
        DataFrame::new(columns)
    }
}

// Add a conpub structor for TopNLevelsDataframeBuilder
impl<const N: usize> TopNLevelsDataframeBuilder<N> {
    pub fn new(capacity: usize) -> Self {
        Self {
            bid_prices: std::array::from_fn(|i| {
                PrimitiveChunkedBuilder::new(
                    PlSmallStr::from(format!("bid_price_{}", i + 1)),
                    capacity,
                )
            }),
            bid_qtys: std::array::from_fn(|i| {
                PrimitiveChunkedBuilder::new(
                    PlSmallStr::from(format!("bid_qty_{}", i + 1)),
                    capacity,
                )
            }),
            ask_prices: std::array::from_fn(|i| {
                PrimitiveChunkedBuilder::new(
                    PlSmallStr::from(format!("ask_price_{}", i + 1)),
                    capacity,
                )
            }),
            ask_qtys: std::array::from_fn(|i| {
                PrimitiveChunkedBuilder::new(
                    PlSmallStr::from(format!("ask_qty_{}", i + 1)),
                    capacity,
                )
            }),
        }
    }
}
