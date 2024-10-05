# Polars Order Book

Polars Order Book is a plugin for the Polars library that efficiently calculates and aligns summary information (price and quantity) for the top N levels of an order book.

## Features

- **Top N Levels Calculation**: Compute the top N price levels for both bid and ask sides of the order book, including quantities.
- **High Performance**: Designed with performance in mind.
- **Flexible Input Formats**: Supports various types of order book updates:
  - Price level updates: `(side, price, updated_quantity)`
  - Order mutations: `(side, price, quantity_change)`
  - Order mutations with modifications: `(side, price, quantity, prev_price, prev_quantity)`

## Usage

Here are examples of how to use the Polars Order Book plugin:

### Example 1: Price Level Updates

```python
import polars as pl
from polars_order_book import top_n_levels_from_price_updates
df = pl.DataFrame({
"price": [10, 10, 11],
"qty": [100, 200, 0],
"is_bid": [True, True, True]
})
expr = top_n_levels_from_price_updates(
price=df["price"],
qty=df["qty"],
is_bid=df["is_bid"],
n=2
)
result = df.with_columns(expr.alias("top_levels")).unnest("top_levels")
print(result)
```

### Example 2: Order Mutations

```python
import polars as pl
from polars_order_book import top_n_levels_from_price_mutations
df = pl.DataFrame({
"price": [100, 101, 102],
"qty": [10, 15, -5],
"is_bid": [True, True, False]
})
expr = top_n_levels_from_price_mutations(
price=df["price"],
qty=df["qty"],
is_bid=df["is_bid"],
n=2
)
result = df.with_columns(expr.alias("top_levels")).unnest("top_levels")
print(result)
```

### Example 3: Order Mutations with Modifications

```python
import polars as pl
from polars_order_book import top_n_levels_from_price_mutations_with_modify
df = pl.DataFrame({
"price": [100, 101, 102],
"qty": [10, 15, 5],
"is_bid": [True, True, False],
"prev_price": [None, 100, None],
"prev_qty": [None, 10, None]
})
expr = top_n_levels_from_price_mutations_with_modify(
df["price"], df["qty"], df["is_bid"], df["prev_price"], df["prev_qty"], n=2
)
result = df.with_columns(expr.alias("top_levels")).unnest("top_levels")
print(result)
```
