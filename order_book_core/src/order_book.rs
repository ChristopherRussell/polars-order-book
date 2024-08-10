use crate::book_side::BookSide;
use crate::book_side_ops::{
    PricePointMutationOps, PricePointMutationOpsError, PricePointSummaryOps,
};
use crate::price_level::{AskPrice, BidPrice, PriceLike, QuantityLike};
pub trait BidAskBook<Px: PriceLike, Qty: QuantityLike> {
    type AskBookSide: BookSide<AskPrice<Px>, Qty>;
    type BidBookSide: BookSide<BidPrice<Px>, Qty>;

    fn asks(&self) -> &Self::AskBookSide;
    fn bids(&self) -> &Self::BidBookSide;
    fn asks_mut(&mut self) -> &mut Self::AskBookSide;
    fn bids_mut(&mut self) -> &mut Self::BidBookSide;
}

pub trait PricePointMutationBookOps<Px: PriceLike, Qty: QuantityLike>:
    BidAskBook<
    Px,
    Qty,
    AskBookSide: PricePointMutationOps<AskPrice<Px>, Qty>,
    BidBookSide: PricePointMutationOps<BidPrice<Px>, Qty>,
>
{
    fn add_qty(&mut self, is_bid: bool, price: Px, qty: Qty) {
        match is_bid {
            true => self.bids_mut().add_qty(BidPrice(price), qty),
            false => self.asks_mut().add_qty(AskPrice(price), qty),
        };
    }

    fn delete_qty(
        &mut self,
        is_bid: bool,
        price: Px,
        qty: Qty,
    ) -> Result<(), PricePointMutationOpsError> {
        match is_bid {
            true => self.bids_mut().delete_qty(BidPrice(price), qty),
            false => self.asks_mut().delete_qty(AskPrice(price), qty),
        }?;
        Ok(())
    }

    fn modify_qty(
        &mut self,
        is_bid: bool,
        prev_price: Px,
        prev_qty: Qty,
        new_price: Px,
        new_qty: Qty,
    ) -> Result<(), PricePointMutationOpsError> {
        self.delete_qty(is_bid, prev_price, prev_qty)?;
        self.add_qty(is_bid, new_price, new_qty);
        Ok(())
    }
}

pub trait PricePointSummaryBookOps<Px: PriceLike, Qty: QuantityLike>:
    BidAskBook<
    Px,
    Qty,
    AskBookSide: PricePointSummaryOps<AskPrice<Px>, Qty>,
    BidBookSide: PricePointSummaryOps<BidPrice<Px>, Qty>,
>
{
    fn set_level(&mut self, is_bid: bool, price: Px, qty: Qty) {
        match is_bid {
            true => self.bids_mut().set_level(BidPrice(price), qty),
            false => self.asks_mut().set_level(AskPrice(price), qty),
        };
    }
}

impl<Px, Qty, Book> PricePointMutationBookOps<Px, Qty> for Book
where
    Px: PriceLike,
    Qty: QuantityLike,
    Book: BidAskBook<Px, Qty>,
    Book::AskBookSide: PricePointMutationOps<AskPrice<Px>, Qty>,
    Book::BidBookSide: PricePointMutationOps<BidPrice<Px>, Qty>,
{
}

impl<Px, Qty, Book> PricePointSummaryBookOps<Px, Qty> for Book
where
    Px: PriceLike,
    Qty: QuantityLike,
    Book: BidAskBook<Px, Qty>,
    Book::AskBookSide: PricePointSummaryOps<AskPrice<Px>, Qty>,
    Book::BidBookSide: PricePointSummaryOps<BidPrice<Px>, Qty>,
{
}

/// Methods for making tests more ergonomic.
pub trait BidAskBookTestMethods<Px, Qty> {
    fn get_level_qty(&self, is_bid: bool, price: Px) -> Option<Qty>;
}

impl<Px, Qty, Book> BidAskBookTestMethods<Px, Qty> for Book
where
    Px: PriceLike,
    Qty: QuantityLike,
    Book: BidAskBook<Px, Qty>,
{
    #[inline]
    fn get_level_qty(&self, is_bid: bool, price: Px) -> Option<Qty> {
        match is_bid {
            true => self.bids().get_level_qty(&BidPrice(price)).copied(),
            false => self.asks().get_level_qty(&AskPrice(price)).copied(),
        }
    }
}
