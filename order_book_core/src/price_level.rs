use num::traits::{Num, NumAssignOps};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{Add, Sub};

/// The trait bound for the inner values of the BidPrice and AskPrice tuple structs.
pub trait PriceLike: Copy + Debug + Hash + Eq + PartialEq + Ord + PartialOrd {}

impl<T> PriceLike for T where T: Copy + Debug + Hash + Eq + PartialEq + Ord + PartialOrd {}

/// The trait bound for PriceTypes, which are BidPrice and AskPrice.
///
/// By encoding the side in the type system we can prevent errors where a
/// BidPrice is used as an AskPrice or vice versa, as well improve performance
/// by doing more work at compile time.
pub trait Price: PriceLike {
    type PriceType: PriceLike;

    const SIDE_NAME: &'static str;
    const IS_BID: bool;

    fn value(&self) -> Self::PriceType;
}

impl<T: PriceLike> From<T> for BidPrice<T> {
    fn from(price: T) -> Self {
        BidPrice(price)
    }
}

impl<T: PriceLike> From<T> for AskPrice<T> {
    fn from(price: T) -> Self {
        AskPrice(price)
    }
}

pub trait QuantityLike:
    Num + NumAssignOps + Clone + Copy + Debug + Eq + PartialEq + Ord + PartialOrd
{
}

impl<T> QuantityLike for T where
    T: Num + NumAssignOps + Clone + Copy + Debug + Eq + PartialEq + Ord + PartialOrd
{
}

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct BidPrice<T>(pub T);

#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct AskPrice<T>(pub T);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum BidOrAskPrice<T: PriceLike> {
    Bid(BidPrice<T>),
    Ask(AskPrice<T>),
}

impl<T: PriceLike> Price for BidPrice<T> {
    type PriceType = T;

    const SIDE_NAME: &'static str = "Bid";
    const IS_BID: bool = true;

    fn value(&self) -> Self::PriceType {
        self.0
    }
}

impl<T: PriceLike> Price for AskPrice<T> {
    type PriceType = T;

    const SIDE_NAME: &'static str = "Ask";
    const IS_BID: bool = false;

    fn value(&self) -> Self::PriceType {
        self.0
    }
}

impl<T: PriceLike> PartialOrd for AskPrice<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: PriceLike> Ord for AskPrice<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0).reverse()
    }
}

impl<T: PriceLike + Num> Add for BidPrice<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        BidPrice(self.0 + rhs.0)
    }
}

impl<T: PriceLike + Num> Add for AskPrice<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        AskPrice(self.0 + rhs.0)
    }
}

impl<T: PriceLike + Num> Sub for BidPrice<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        BidPrice(self.0 - rhs.0)
    }
}

impl<T: PriceLike + Num> Sub for AskPrice<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        AskPrice(self.0 - rhs.0)
    }
}

#[derive(Eq, PartialEq, Clone, Copy)]
pub struct PriceLevel<Px: Price, Qty: QuantityLike> {
    pub price: Px,
    pub qty: Qty,
}

impl<Px: Price, Qty: QuantityLike> Debug for PriceLevel<Px, Qty> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} @ {:?}", self.price, self.qty)
    }
}

impl<Px: Price, Qty: QuantityLike> PriceLevel<Px, Qty> {
    #[must_use]
    pub fn new(price: Px) -> Self {
        PriceLevel {
            price,
            qty: Qty::zero(),
        }
    }

    pub fn add_qty(&mut self, qty: Qty) {
        self.qty += qty;
    }

    pub fn delete_qty(&mut self, qty: Qty) {
        self.qty -= qty;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let price_level: PriceLevel<_, i32> = PriceLevel::new(BidPrice(100));
        assert_eq!(price_level.price, 100.into());
        assert_eq!(price_level.qty, 0);

        let price_level: PriceLevel<_, i32> = PriceLevel::new(AskPrice(100));
        assert_eq!(price_level.price, 100.into());
        assert_eq!(price_level.qty, 0);
    }

    #[test]
    fn test_add_qty() {
        let mut price_level = PriceLevel::new(BidPrice(100));
        price_level.add_qty(10);
        assert_eq!(price_level.qty, 10);

        price_level.add_qty(5);
        assert_eq!(price_level.qty, 15);

        let mut price_level = PriceLevel::new(AskPrice(100));
        price_level.add_qty(10);
        assert_eq!(price_level.qty, 10);

        price_level.add_qty(5);
        assert_eq!(price_level.qty, 15);
    }

    #[test]
    fn test_delete_qty() {
        let mut price_level = PriceLevel::new(BidPrice(100));
        price_level.add_qty(15);
        price_level.delete_qty(5);
        assert_eq!(price_level.qty, 10);

        price_level.delete_qty(4);
        assert_eq!(price_level.qty, 6);

        price_level.delete_qty(3);
        assert_eq!(price_level.qty, 3);

        price_level.delete_qty(2);
        assert_eq!(price_level.qty, 1);

        price_level.delete_qty(1);
        assert_eq!(price_level.qty, 0);

        let mut price_level = PriceLevel::new(AskPrice(100));
        price_level.add_qty(15);
        price_level.delete_qty(5);
        assert_eq!(price_level.qty, 10);

        price_level.delete_qty(4);
        assert_eq!(price_level.qty, 6);

        price_level.delete_qty(3);
        assert_eq!(price_level.qty, 3);

        price_level.delete_qty(2);
        assert_eq!(price_level.qty, 1);

        price_level.delete_qty(1);
        assert_eq!(price_level.qty, 0);
    }

    #[test]
    fn test_bid_ordering() {
        let bid_100 = BidPrice(100);
        let bid_200 = BidPrice(200);
        assert!(bid_200 > bid_100);
        assert!(bid_100 < bid_200);
        assert_eq!(bid_100, bid_100);
    }

    #[test]
    fn test_ask_ordering() {
        let ask_100 = AskPrice(100);
        let ask_200 = AskPrice(200);
        assert!(ask_200 < ask_100);
        assert!(ask_100 > ask_200);
        assert_eq!(ask_100, ask_100);
    }
}
