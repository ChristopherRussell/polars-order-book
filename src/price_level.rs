use num::traits::Num;

#[derive(Debug, Eq, PartialEq)]
pub struct PriceLevel<Price, Qty> {
    pub is_bid: bool,
    pub price: Price,
    pub qty: Qty,
}

impl<Price, Qty: Num + Copy> PriceLevel<Price, Qty> {
    #[must_use]
    pub fn new(is_bid: bool, price: Price) -> Self {
        PriceLevel {
            is_bid,
            price,
            qty: Qty::zero(),
        }
    }

    pub fn add_qty(&mut self, qty: Qty) {
        self.qty = self.qty + qty;
    }

    pub fn delete_qty(&mut self, qty: Qty) {
        self.qty = self.qty - qty;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let price_level: PriceLevel<u32, u32> = PriceLevel::new(true, 100);
        assert!(price_level.is_bid);
        assert_eq!(price_level.price, 100);
        assert_eq!(price_level.qty, 0);
    }

    #[test]
    fn test_add_qty() {
        let mut price_level = PriceLevel::new(true, 100);
        price_level.add_qty(10);
        assert_eq!(price_level.qty, 10);

        price_level.add_qty(5);
        assert_eq!(price_level.qty, 15);
    }

    #[test]
    fn test_delete_qty() {
        let mut price_level = PriceLevel::new(true, 100);
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
}
