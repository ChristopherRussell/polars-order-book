# Polars Order Book

Polars Order Book provides plugins for the Polars library that efficiently calculate summary information (price and quantity) for the top N levels of an order book.

## Features

- **Top N Levels**: Compute the price and quantity for the top N price levels of both bid and ask sides of the order book.
- **High Performance**: Designed with performance in mind.
- **Multiple Input Formats**: Supports various types of order book updates:
  - Price level updates: `(side, price, new_quantity)`
  - Order mutations: `(side, price, quantity_change)`
  - Order mutations with modifications: `(side, price, quantity, prev_price, prev_quantity)`

## Usage

Here are examples of how to use the plugin:

### Example 1: Price Level Updates

```python
import polars as pl
from polars_order_book import top_n_levels_from_price_updates

df = pl.DataFrame(
    {
        "is_bid": [True, True, False, False, True, True],
        "price": [1, 2, 4, 5, 2, 2],
        "qty": [100, 200, 400, 500, 250, 0],
    }
)
expr = top_n_levels_from_price_updates(
    price=df["price"], qty=df["qty"], is_bid=df["is_bid"], n=2
)
result = df.with_columns(expr.alias("top_levels")).unnest("top_levels")
print(result)

# Output
shape: (6, 11)
┌────────┬───────┬─────┬─────────────┬─────────────┬───────────┬───────────┬─────────────┬─────────────┬───────────┬───────────┐
│ is_bid ┆ price ┆ qty ┆ bid_price_1 ┆ bid_price_2 ┆ bid_qty_1 ┆ bid_qty_2 ┆ ask_price_1 ┆ ask_price_2 ┆ ask_qty_1 ┆ ask_qty_2 │
│ ---    ┆ ---   ┆ --- ┆ ---         ┆ ---         ┆ ---       ┆ ---       ┆ ---         ┆ ---         ┆ ---       ┆ ---       │
│ bool   ┆ i64   ┆ i64 ┆ i64         ┆ i64         ┆ i64       ┆ i64       ┆ i64         ┆ i64         ┆ i64       ┆ i64       │
╞════════╪═══════╪═════╪═════════════╪═════════════╪═══════════╪═══════════╪═════════════╪═════════════╪═══════════╪═══════════╡
│ true   ┆ 1     ┆ 100 ┆ 1           ┆ null        ┆ 100       ┆ null      ┆ null        ┆ null        ┆ null      ┆ null      │
│ true   ┆ 2     ┆ 200 ┆ 2           ┆ 1           ┆ 200       ┆ 100       ┆ null        ┆ null        ┆ null      ┆ null      │
│ false  ┆ 4     ┆ 400 ┆ 2           ┆ 1           ┆ 200       ┆ 100       ┆ 4           ┆ null        ┆ 400       ┆ null      │
│ false  ┆ 5     ┆ 500 ┆ 2           ┆ 1           ┆ 200       ┆ 100       ┆ 4           ┆ 5           ┆ 400       ┆ 500       │
│ true   ┆ 2     ┆ 250 ┆ 2           ┆ 1           ┆ 250       ┆ 100       ┆ 4           ┆ 5           ┆ 400       ┆ 500       │
│ true   ┆ 2     ┆ 0   ┆ 1           ┆ null        ┆ 100       ┆ null      ┆ 4           ┆ 5           ┆ 400       ┆ 500       │
└────────┴───────┴─────┴─────────────┴─────────────┴───────────┴───────────┴─────────────┴─────────────┴───────────┴───────────┘
```

### Example 2: Order Mutations

```python
import polars as pl
from polars_order_book import top_n_levels_from_price_mutations

df = pl.DataFrame(
    {
        "is_bid": [True, True, False, False, True, True],
        "price": [1, 2, 4, 5, 2, 2],
        "qty": [100, 200, 400, 500, 50, -250],
    }
)
expr = top_n_levels_from_price_mutations(price="price", qty="qty", is_bid="is_bid", n=2)
result = df.with_columns(expr.alias("top_levels")).unnest("top_levels")
print(result)

# Output
shape: (6, 11)
┌────────┬───────┬──────┬─────────────┬─────────────┬───────────┬───────────┬─────────────┬─────────────┬───────────┬───────────┐
│ is_bid ┆ price ┆ qty  ┆ bid_price_1 ┆ bid_price_2 ┆ bid_qty_1 ┆ bid_qty_2 ┆ ask_price_1 ┆ ask_price_2 ┆ ask_qty_1 ┆ ask_qty_2 │
│ ---    ┆ ---   ┆ ---  ┆ ---         ┆ ---         ┆ ---       ┆ ---       ┆ ---         ┆ ---         ┆ ---       ┆ ---       │
│ bool   ┆ i64   ┆ i64  ┆ i64         ┆ i64         ┆ i64       ┆ i64       ┆ i64         ┆ i64         ┆ i64       ┆ i64       │
╞════════╪═══════╪══════╪═════════════╪═════════════╪═══════════╪═══════════╪═════════════╪═════════════╪═══════════╪═══════════╡
│ true   ┆ 1     ┆ 100  ┆ 1           ┆ null        ┆ 100       ┆ null      ┆ null        ┆ null        ┆ null      ┆ null      │
│ true   ┆ 2     ┆ 200  ┆ 2           ┆ 1           ┆ 200       ┆ 100       ┆ null        ┆ null        ┆ null      ┆ null      │
│ false  ┆ 4     ┆ 400  ┆ 2           ┆ 1           ┆ 200       ┆ 100       ┆ 4           ┆ null        ┆ 400       ┆ null      │
│ false  ┆ 5     ┆ 500  ┆ 2           ┆ 1           ┆ 200       ┆ 100       ┆ 4           ┆ 5           ┆ 400       ┆ 500       │
│ true   ┆ 2     ┆ 50   ┆ 2           ┆ 1           ┆ 250       ┆ 100       ┆ 4           ┆ 5           ┆ 400       ┆ 500       │
│ true   ┆ 2     ┆ -250 ┆ 1           ┆ null        ┆ 100       ┆ null      ┆ 4           ┆ 5           ┆ 400       ┆ 500       │
└────────┴───────┴──────┴─────────────┴─────────────┴───────────┴───────────┴─────────────┴─────────────┴───────────┴───────────┘
```

### Example 3: Order Mutations with Modifications

```python
import polars as pl
from polars_order_book import top_n_levels_from_price_mutations_with_modify

df = pl.DataFrame(
    {
        "is_bid": [True, False, True, False, True, False],
        "price": [1, 6, 2, 5, 3, 4],
        "qty": [100, 600, 200, 500, 300, 400],
        "prev_price": [None, None, 1, 6, 2, 5],
        "prev_qty": [None, None, 100, 600, 200, 500],
    }
)
expr = top_n_levels_from_price_mutations_with_modify(
    "price", "qty", "is_bid", "prev_price", "prev_qty", n=2
)
result = df.with_columns(expr.alias("top_levels")).unnest("top_levels")
print(result)

# Output
shape: (6, 13)
┌────────┬───────┬─────┬────────────┬──────────┬─────────────┬─────────────┬───────────┬───────────┬─────────────┬─────────────┬───────────┬───────────┐
│ is_bid ┆ price ┆ qty ┆ prev_price ┆ prev_qty ┆ bid_price_1 ┆ bid_price_2 ┆ bid_qty_1 ┆ bid_qty_2 ┆ ask_price_1 ┆ ask_price_2 ┆ ask_qty_1 ┆ ask_qty_2 │
│ ---    ┆ ---   ┆ --- ┆ ---        ┆ ---      ┆ ---         ┆ ---         ┆ ---       ┆ ---       ┆ ---         ┆ ---         ┆ ---       ┆ ---       │
│ bool   ┆ i64   ┆ i64 ┆ i64        ┆ i64      ┆ i64         ┆ i64         ┆ i64       ┆ i64       ┆ i64         ┆ i64         ┆ i64       ┆ i64       │
╞════════╪═══════╪═════╪════════════╪══════════╪═════════════╪═════════════╪═══════════╪═══════════╪═════════════╪═════════════╪═══════════╪═══════════╡
│ true   ┆ 1     ┆ 100 ┆ null       ┆ null     ┆ 1           ┆ null        ┆ 100       ┆ null      ┆ null        ┆ null        ┆ null      ┆ null      │
│ false  ┆ 6     ┆ 600 ┆ null       ┆ null     ┆ 1           ┆ null        ┆ 100       ┆ null      ┆ 6           ┆ null        ┆ 600       ┆ null      │
│ true   ┆ 2     ┆ 200 ┆ 1          ┆ 100      ┆ 2           ┆ null        ┆ 200       ┆ null      ┆ 6           ┆ null        ┆ 600       ┆ null      │
│ false  ┆ 5     ┆ 500 ┆ 6          ┆ 600      ┆ 2           ┆ null        ┆ 200       ┆ null      ┆ 5           ┆ null        ┆ 500       ┆ null      │
│ true   ┆ 3     ┆ 300 ┆ 2          ┆ 200      ┆ 3           ┆ null        ┆ 300       ┆ null      ┆ 5           ┆ null        ┆ 500       ┆ null      │
│ false  ┆ 4     ┆ 400 ┆ 5          ┆ 500      ┆ 3           ┆ null        ┆ 300       ┆ null      ┆ 4           ┆ null        ┆ 400       ┆ null      │
└────────┴───────┴─────┴────────────┴──────────┴─────────────┴─────────────┴───────────┴───────────┴─────────────┴─────────────┴───────────┴───────────┘
```

## Practical Considerations and Tips

### Converting Exchange Messages to Mutations

In practice, you may need to modify the order book data you have to get it into one of the supported input formats. The following example demonstrates several common modifications:

- Convert `side` column to an `is_bid` boolean
- Convert float `price` column to integers for internal processing, and convert output prices back to float
- Represent deletes and trades as negative quantity mutations

```python
import polars as pl
from polars_order_book import top_n_levels_from_price_mutations

messages = pl.DataFrame(
    {
        "message_type": ["add", "add", "add", "add", "trade", "delete"],
        "side": ["bid", "bid", "ask", "ask", "bid", "bid"],
        "price": [0.01, 0.02, 0.04, 0.05, 0.02, 0.02],
        "qty": [100, 200, 400, 500, 50, 150],
    }
)
PRICE_FACTOR = 100
mutations = messages.lazy().select(
    is_bid=pl.col("side") == "bid",
    price=(pl.col("price") * PRICE_FACTOR).round().cast(pl.Int64),
    qty=pl.when(pl.col("message_type").is_in(["delete", "trade"]))
    .then(-pl.col("qty"))
    .otherwise(pl.col("qty")),
)
expr = top_n_levels_from_price_mutations(price="price", qty="qty", is_bid="is_bid", n=2)
top_levels = (
    mutations.with_columns(top_levels=expr)
    .select("top_levels")
    .unnest("top_levels")
    .with_columns(pl.selectors.matches(r"^(bid|ask)_price_\d+$") / PRICE_FACTOR)  # Cast prices back to floats
    .collect()
)
result = pl.concat([messages, top_levels], how="horizontal")
print(result)

# Output
shape: (6, 12)
┌──────────────┬──────┬───────┬─────┬─────────────┬─────────────┬───────────┬───────────┬─────────────┬─────────────┬───────────┬───────────┐
│ message_type ┆ side ┆ price ┆ qty ┆ bid_price_1 ┆ bid_price_2 ┆ bid_qty_1 ┆ bid_qty_2 ┆ ask_price_1 ┆ ask_price_2 ┆ ask_qty_1 ┆ ask_qty_2 │
│ ---          ┆ ---  ┆ ---   ┆ --- ┆ ---         ┆ ---         ┆ ---       ┆ ---       ┆ ---         ┆ ---         ┆ ---       ┆ ---       │
│ str          ┆ str  ┆ f64   ┆ i64 ┆ f64         ┆ f64         ┆ i64       ┆ i64       ┆ f64         ┆ f64         ┆ i64       ┆ i64       │
╞══════════════╪══════╪═══════╪═════╪═════════════╪═════════════╪═══════════╪═══════════╪═════════════╪═════════════╪═══════════╪═══════════╡
│ add          ┆ bid  ┆ 0.01  ┆ 100 ┆ 0.01        ┆ null        ┆ 100       ┆ null      ┆ null        ┆ null        ┆ null      ┆ null      │
│ add          ┆ bid  ┆ 0.02  ┆ 200 ┆ 0.02        ┆ 0.01        ┆ 200       ┆ 100       ┆ null        ┆ null        ┆ null      ┆ null      │
│ add          ┆ ask  ┆ 0.04  ┆ 400 ┆ 0.02        ┆ 0.01        ┆ 200       ┆ 100       ┆ 0.04        ┆ null        ┆ 400       ┆ null      │
│ add          ┆ ask  ┆ 0.05  ┆ 500 ┆ 0.02        ┆ 0.01        ┆ 200       ┆ 100       ┆ 0.04        ┆ 0.05        ┆ 400       ┆ 500       │
│ trade        ┆ bid  ┆ 0.02  ┆ 50  ┆ 0.02        ┆ 0.01        ┆ 150       ┆ 100       ┆ 0.04        ┆ 0.05        ┆ 400       ┆ 500       │
│ delete       ┆ bid  ┆ 0.02  ┆ 150 ┆ 0.01        ┆ null        ┆ 100       ┆ null      ┆ 0.04        ┆ 0.05        ┆ 400       ┆ 500       │
└──────────────┴──────┴───────┴─────┴─────────────┴─────────────┴───────────┴───────────┴─────────────┴─────────────┴───────────┴───────────┘
```

### Potential Pitfalls

1. **Unsorted Data**: Messages must be processed in the correct order. Always sort your data by timestamp or sequence number before applying order book calculations.

2. **Multiple Products**: For datasets containing multiple products, apply the `top_n_level_*` expression in a group-by context. For example:

   ```python
   ...
   result = (
       mutations.group_by("product_id")
       .agg(
           top_levels=top_n_levels_from_price_mutations(
               price="price", qty="qty", is_bid="is_bid", n=2
           )
       )
       .unnest("top_levels")
   )
   ```

3. **Repeated Information**: Ensure each mutation is applied only once. For instance, if you have both `trade` (one per passive order executed) and `trade_summary` (one per aggressive order) messages, discard the `trade_summary` to avoid double-counting.

4. **Order Book Resets**: Your dataset may includes periods where the order book is cleared without explicit delete messages for all orders. To handle this:
   - Add a `reset_count` column to your data, incrementing it each time the book is reset.
   - Apply the `top_n_level_*` expression with a group-by on this column:

   ```python
   ...
   result = (
       mutations.group_by(["product_id", "reset_count"])
       .agg(
           top_levels=top_n_levels_from_price_mutations(
               price="price", qty="qty", is_bid="is_bid", n=2
           )
       )
       .unnest("top_levels")
   )
   ```

![Polar Order Bear](polar_order_bear.jpg)