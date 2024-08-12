from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import polars as pl

from polars_order_book._internal import __version__ as __version__
from polars_order_book.utils import parse_into_expr, parse_version, register_plugin

if TYPE_CHECKING:
    from polars.type_aliases import IntoExpr

if parse_version(pl.__version__) < parse_version("0.20.16"):
    from polars.utils.udfs import _get_shared_lib_location

    lib: str | Path = _get_shared_lib_location(__file__)
else:
    lib = Path(__file__).parent


def top_n_levels_from_price_mutations(
    price: IntoExpr,
    qty: IntoExpr,
    is_bid: IntoExpr,
    prev_price: IntoExpr | None = None,
    prev_qty: IntoExpr | None = None,
    *,
    n: int = 1,
) -> pl.Expr:
    """
    Calculate the top `n` levels of the bid and ask sides of the order book from price mutations.

    This function processes price mutations such as additions, deletions, and (optionally) modifications.
    - An addition is represented by a positive quantity, e.g., `{price: 100, qty: 10, is_bid: True}` adds 10 lots to the bid price of 100.
    - A deletion is represented by a negative quantity, e.g., `{price: 100, qty: -10, is_bid: True}` removes 10 lots from the bid price of 100.
    - A price modification of an order is specified by including the previous price and quantity as well as the new price and quantity, e.g.,
      `{price: 100, qty: 10, is_bid: True, prev_price: 99, prev_qty: 10}` modifies an order from 99 lots at 99 to 10 lots at 100.
    - A quantity-only modification can be represented with `prev_price` as None, or with `prev_price` being the same as `price`.

    Parameters
    ----------
    price : IntoExpr
        The price levels to be updated.
    qty : IntoExpr
        The corresponding quantities for each price level.
    is_bid : IntoExpr
        Boolean flag indicating whether the price level is on the bid side (`True`) or ask side (`False`).
    prev_price : IntoExpr or None, optional
        The previous price level for modifications. If provided, `prev_qty` must also be provided. Default is None.
    prev_qty : IntoExpr or None, optional
        The previous quantity for modifications. If provided, `prev_price` must also be provided. Default is None.
    n : int, optional
        The number of top levels to calculate. Default is 1.

    Returns
    -------
    pl.Expr
        A Polars expression that can be used to compute the top `n` levels of the order book.

    Examples
    --------
    >>> import polars as pl
    >>> df = pl.DataFrame({
    ...     "price": [100, 101, 102],
    ...     "qty": [10, 15, 5],
    ...     "is_bid": [True, True, False]
    ... })
    >>> expr = top_n_levels_from_price_mutations(df["price"], df["qty"], df["is_bid"], n=1)
    >>> df.with_columns(expr.alias("top_levels"))
    """
    price = parse_into_expr(price)
    qty = parse_into_expr(qty)
    is_bid = parse_into_expr(is_bid)
    if (prev_price is not None) and (prev_qty is not None):
        prev_price = parse_into_expr(prev_price)
        prev_qty = parse_into_expr(prev_qty)
        args = [price, qty, is_bid, prev_price, prev_qty]
    elif (prev_price is None) and (prev_qty is None):
        args = [price, qty, is_bid]
    else:
        raise ValueError(
            f"""Cannot provide only one of prev_price and prev_qty. Got:\n
            prev_price={prev_price},\nprev_qty={prev_qty}"""
        )

    return register_plugin(
        args=args,  # type: ignore
        symbol="pl_calculate_bbo",
        is_elementwise=False,
        lib=lib,
        kwargs={"n": n},
    )


def top_n_levels_from_price_updates(
    price: IntoExpr,
    qty: IntoExpr,
    is_bid: IntoExpr,
    *,
    n: int = 1,
) -> pl.Expr:
    """
    Calculate the top `n` levels of the bid and ask sides of the order book from price updates.

    This function processes book updates where a new snapshot of a price level replaces the old level.
    For example, if the order book is empty:
    - `{price: 10, qty: 100, is_bid: True}` sets the quantity to 100 at price 10.
    - `{price: 10, qty: 200, is_bid: True}` sets the quantity to 200.
    - `{price: 10, qty: 0, is_bid: True}` removes the level.

    The important distinction is that the quantity in this function replaces the old quantity, as opposed to being added/subtracted like in `top_n_levels_from_price_mutations`.

    Parameters
    ----------
    price : IntoExpr
        The price levels to be updated.
    qty : IntoExpr
        The corresponding quantities for each price level.
    is_bid : IntoExpr
        Boolean flag indicating whether the price level is on the bid side (`True`) or ask side (`False`).
    n : int, optional
        The number of top levels to calculate. Default is 1.

    Returns
    -------
    pl.Expr
        A Polars expression that can be used to compute the top `n` levels of the order book.

    Examples
    --------
    >>> import polars as pl
    >>> df = pl.DataFrame({
    ...     "price": [10, 10, 10],
    ...     "qty": [100, 200, 0],
    ...     "is_bid": [True, True, True]
    ... })
    >>> expr = top_n_levels_from_price_updates(df["price"], df["qty"], df["is_bid"], n=1)
    >>> df.with_columns(expr.alias("top_levels"))
    """
    price = parse_into_expr(price)
    qty = parse_into_expr(qty)
    is_bid = parse_into_expr(is_bid)
    args = [price, qty, is_bid]

    return register_plugin(
        args=args,  # type: ignore
        symbol="pl_calculate_bbo",
        is_elementwise=False,
        lib=lib,
        kwargs={"n": n},
    )
