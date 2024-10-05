use order_book_core::book_side_ops::PricePointMutationOpsError;
use order_book_core::{
    order_book::{BidAskBook, PricePointMutationBookOps, PricePointSummaryBookOps},
    price_level,
};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum UpdateMissingValueError {
    #[error("is_bid should not be None")]
    IsBid,
    #[error("price should not be None")]
    Price,
    #[error("quantity should not be None")]
    Quantity,
}

#[derive(Debug)]
pub struct PriceUpdate<Px: price_level::PriceLike, Qty: price_level::QuantityLike> {
    pub is_bid: bool,
    pub price: Px,
    pub quantity: Qty,
}

impl<Px: price_level::PriceLike, Qty: price_level::QuantityLike> PriceUpdate<Px, Qty> {
    pub fn try_new(
        is_bid: Option<bool>,
        price: Option<Px>,
        quantity: Option<Qty>,
    ) -> Result<Self, UpdateMissingValueError> {
        Ok(PriceUpdate {
            is_bid: is_bid.ok_or(UpdateMissingValueError::IsBid)?,
            price: price.ok_or(UpdateMissingValueError::Price)?,
            quantity: quantity.ok_or(UpdateMissingValueError::Quantity)?,
        })
    }
}

#[derive(Debug)]
pub struct PriceMutation<Px: price_level::PriceLike, Qty: price_level::QuantityLike> {
    pub is_bid: bool,
    pub price: Px,
    pub quantity: Qty,
}

impl<Px: price_level::PriceLike, Qty: price_level::QuantityLike> PriceMutation<Px, Qty> {
    pub fn try_new(
        is_bid: Option<bool>,
        price: Option<Px>,
        quantity: Option<Qty>,
    ) -> Result<Self, UpdateMissingValueError> {
        Ok(PriceMutation {
            is_bid: is_bid.ok_or(UpdateMissingValueError::IsBid)?,
            price: price.ok_or(UpdateMissingValueError::Price)?,
            quantity: quantity.ok_or(UpdateMissingValueError::Quantity)?,
        })
    }
}
#[derive(Debug)]
pub struct PriceMutationWithModify<Px: price_level::PriceLike, Qty: price_level::QuantityLike> {
    pub is_bid: bool,
    pub price: Px,
    pub quantity: Qty,
    pub prev_price: Option<Px>,
    pub prev_quantity: Option<Qty>,
}

impl<Px: price_level::PriceLike, Qty: price_level::QuantityLike> PriceMutationWithModify<Px, Qty> {
    pub fn try_new(
        is_bid: Option<bool>,
        price: Option<Px>,
        quantity: Option<Qty>,
        prev_price: Option<Px>,
        prev_quantity: Option<Qty>,
    ) -> Result<Self, UpdateMissingValueError> {
        Ok(PriceMutationWithModify {
            is_bid: is_bid.ok_or(UpdateMissingValueError::IsBid)?,
            price: price.ok_or(UpdateMissingValueError::Price)?,
            quantity: quantity.ok_or(UpdateMissingValueError::Quantity)?,
            prev_price,
            prev_quantity,
        })
    }
}

pub trait ApplyUpdate<Px, Qty, Book>
where
    Px: price_level::PriceLike,
    Qty: price_level::QuantityLike,
    Book: BidAskBook<Px, Qty>,
{
    fn apply_update(self, book: &mut Book) -> Result<(), PricePointMutationOpsError>;
}

impl<
        Px: price_level::PriceLike,
        Qty: price_level::QuantityLike,
        Book: PricePointSummaryBookOps<Px, Qty>,
    > ApplyUpdate<Px, Qty, Book> for PriceUpdate<Px, Qty>
{
    fn apply_update(self, book: &mut Book) -> Result<(), PricePointMutationOpsError> {
        book.set_level(self.is_bid, self.price, self.quantity);
        Ok(())
    }
}

impl<
        Px: price_level::PriceLike,
        Qty: price_level::QuantityLike + num::Signed,
        Book: PricePointMutationBookOps<Px, Qty>,
    > ApplyUpdate<Px, Qty, Book> for PriceMutation<Px, Qty>
{
    fn apply_update(self, book: &mut Book) -> Result<(), PricePointMutationOpsError> {
        match self.quantity.cmp(&Qty::zero()) {
            std::cmp::Ordering::Less => {
                book.delete_qty(self.is_bid, self.price, self.quantity.abs())?
            }
            std::cmp::Ordering::Greater => book.add_qty(self.is_bid, self.price, self.quantity),
            // Adding could create a level, deleting could fail if level doesn't exist, so safest to do nothing.
            std::cmp::Ordering::Equal => {}
        }
        Ok(())
    }
}

impl<
        Px: price_level::PriceLike,
        Qty: price_level::QuantityLike + num::Signed,
        Book: PricePointMutationBookOps<Px, Qty>,
    > ApplyUpdate<Px, Qty, Book> for PriceMutationWithModify<Px, Qty>
{
    fn apply_update(self, book: &mut Book) -> Result<(), PricePointMutationOpsError> {
        match (
            self.quantity.cmp(&Qty::zero()),
            self.prev_quantity,
            self.prev_price,
        ) {
            (std::cmp::Ordering::Greater, Some(prev_quantity), Some(prev_price)) => book.modify_qty(
                self.is_bid,
                prev_price,
                prev_quantity,
                self.price,
                self.quantity,
            )?,

            (std::cmp::Ordering::Greater, None, None) => book.add_qty(self.is_bid, self.price, self.quantity),
            (std::cmp::Ordering::Less, None, None) => book.delete_qty(self.is_bid, self.price, self.quantity.abs())?,
            (std::cmp::Ordering::Greater, Some(prev_quantity), None) => {
                book.delete_qty(self.is_bid, self.price, prev_quantity - self.quantity.abs())?;
            }
            (std::cmp::Ordering::Equal, None, None) => {}
            (std::cmp::Ordering::Equal, Some(prev_quantity), None) => {
                book.delete_qty(self.is_bid, self.price, prev_quantity)?;
            }
            (std::cmp::Ordering::Equal, Some(prev_quantity), Some(prev_price)) => {
                book.delete_qty(self.is_bid, prev_price, prev_quantity)?;
            }
            (std::cmp::Ordering::Less, _, _) => panic!("Quantity should not be negative for a mutation where prev_quantity and/or prev_price are non-null: {:?}", self),
            (_, None, Some(_)) => panic!("prev_quantity must not be null when prev_price is not null: {:?}", self),
        }
        Ok(())
    }
}

pub struct PriceUpdateIterator<'a> {
    is_bid: Box<dyn Iterator<Item = Option<bool>> + 'a>,
    price: Box<dyn Iterator<Item = Option<i64>> + 'a>,
    quantity: Box<dyn Iterator<Item = Option<i64>> + 'a>,
}

impl<'a> PriceUpdateIterator<'a> {
    pub fn new(
        is_bid: impl Iterator<Item = Option<bool>> + 'a,
        price: impl Iterator<Item = Option<i64>> + 'a,
        quantity: impl Iterator<Item = Option<i64>> + 'a,
    ) -> Self {
        PriceUpdateIterator {
            is_bid: Box::new(is_bid),
            price: Box::new(price),
            quantity: Box::new(quantity),
        }
    }
}

impl Iterator for PriceUpdateIterator<'_> {
    type Item = Result<PriceUpdate<i64, i64>, UpdateMissingValueError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(is_bid) = self.is_bid.next() {
            Some(PriceUpdate::try_new(
                is_bid,
                self.price
                    .next()
                    .expect("Input series should be same length"),
                self.quantity
                    .next()
                    .expect("Input series should be same length"),
            ))
        } else {
            None
        }
    }
}

pub struct PriceMutationIterator<'a> {
    is_bid: Box<dyn Iterator<Item = Option<bool>> + 'a>,
    price: Box<dyn Iterator<Item = Option<i64>> + 'a>,
    quantity: Box<dyn Iterator<Item = Option<i64>> + 'a>,
}

impl<'a> PriceMutationIterator<'a> {
    pub fn new(
        is_bid: impl Iterator<Item = Option<bool>> + 'a,
        price: impl Iterator<Item = Option<i64>> + 'a,
        quantity: impl Iterator<Item = Option<i64>> + 'a,
    ) -> Self {
        PriceMutationIterator {
            is_bid: Box::new(is_bid),
            price: Box::new(price),
            quantity: Box::new(quantity),
        }
    }
}

impl Iterator for PriceMutationIterator<'_> {
    type Item = Result<PriceMutation<i64, i64>, UpdateMissingValueError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(is_bid) = self.is_bid.next() {
            Some(PriceMutation::try_new(
                is_bid,
                self.price
                    .next()
                    .expect("Input series should be same length"),
                self.quantity
                    .next()
                    .expect("Input series should be same length"),
            ))
        } else {
            None
        }
    }
}

pub struct PriceMutationWithModifyIterator<'a> {
    is_bid: Box<dyn Iterator<Item = Option<bool>> + 'a>,
    price: Box<dyn Iterator<Item = Option<i64>> + 'a>,
    quantity: Box<dyn Iterator<Item = Option<i64>> + 'a>,
    prev_price: Box<dyn Iterator<Item = Option<i64>> + 'a>,
    prev_quantity: Box<dyn Iterator<Item = Option<i64>> + 'a>,
}

impl<'a> PriceMutationWithModifyIterator<'a> {
    pub fn new(
        is_bid: impl Iterator<Item = Option<bool>> + 'a,
        price: impl Iterator<Item = Option<i64>> + 'a,
        quantity: impl Iterator<Item = Option<i64>> + 'a,
        prev_price: impl Iterator<Item = Option<i64>> + 'a,
        prev_quantity: impl Iterator<Item = Option<i64>> + 'a,
    ) -> Self {
        PriceMutationWithModifyIterator {
            is_bid: Box::new(is_bid),
            price: Box::new(price),
            quantity: Box::new(quantity),
            prev_price: Box::new(prev_price),
            prev_quantity: Box::new(prev_quantity),
        }
    }
}

impl Iterator for PriceMutationWithModifyIterator<'_> {
    type Item = Result<PriceMutationWithModify<i64, i64>, UpdateMissingValueError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(is_bid) = self.is_bid.next() {
            Some(PriceMutationWithModify::try_new(
                is_bid,
                self.price
                    .next()
                    .expect("Input series should be same length"),
                self.quantity
                    .next()
                    .expect("Input series should be same length"),
                self.prev_price
                    .next()
                    .expect("Input series should be same length"),
                self.prev_quantity
                    .next()
                    .expect("Input series should be same length"),
            ))
        } else {
            None
        }
    }
}
