from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import polars as pl

from polars_order_book._internal import __version__ as __version__
from polars_order_book._utils import parse_into_expr, parse_version, register_plugin

if TYPE_CHECKING:
    from polars.type_aliases import IntoExpr

if parse_version(pl.__version__) < parse_version("0.20.16"):
    from polars.utils.udfs import _get_shared_lib_location

    lib: str | Path = _get_shared_lib_location(__file__)
else:
    lib = Path(__file__).parent


def top_n_levels_from_price_mutations_with_modify(
    price: IntoExpr,
    qty: IntoExpr,
    is_bid: IntoExpr,
    prev_price: IntoExpr | None = None,
    prev_qty: IntoExpr | None = None,
    *,
    n: int = 1,
) -> pl.Expr:
    """
    Calculate the top `n` levels of the bid and ask sides of the order book from price mutations with modifications.

    This function processes price mutations including additions, deletions, and modifications.
    - An addition is represented by positive quantities with no previous price and quantity.
    - A deletion is represented by negative quantities with no previous price and quantity.
    - A modification is specified by including both the current and previous price and quantity.
    - A quantity-only modify may be represented by providing price, quantity and prev_qty without prev_price,
      though this is more simply represented by an addition or a deletion.

    Parameters
    ----------
    price : IntoExpr
        The price levels for the current update.
    qty : IntoExpr
        The quantities for the current update.
    is_bid : IntoExpr
        Boolean flag indicating whether the price level is on the bid side (True) or ask side (False).
    prev_price : IntoExpr or None, optional
        The previous price levels for modifications. If provided, `prev_qty` must also be provided.
    prev_qty : IntoExpr or None, optional
        The previous quantities for modifications.
    n : int, optional
        The number of top levels to calculate. Default is 1.

    Returns
    -------
    pl.Expr
        A Polars expression that computes the top `n` levels of the order book.

    Examples
    --------
    >>> import polars as pl
    >>> from polars_order_book import top_n_levels_from_price_mutations_with_modify
    >>> df = pl.DataFrame({
    ...     "price": [100, 101, 102],
    ...     "qty": [10, 15, 5],
    ...     "is_bid": [True, True, False],
    ...     "prev_price": [None, 100, None],
    ...     "prev_qty": [None, 10, None]
    ... })
    >>> expr = top_n_levels_from_price_mutations_with_modify(
    ...     df["price"], df["qty"], df["is_bid"], df["prev_price"], df["prev_qty"], n=2
    ... )
    >>> df.with_columns(expr.alias("top_levels")).unnest("top_levels")
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
            f"Cannot provide only one of prev_price and prev_qty. Got:\n"
            f"prev_price={prev_price},\nprev_qty={prev_qty}"
        )

    return register_plugin(
        args=args,  # type: ignore
        symbol="pl_calculate_bbo_mutation_modify",
        is_elementwise=False,
        lib=lib,
        kwargs={"n": n},
    )


def top_n_levels_from_price_mutations(
    price: IntoExpr,
    qty: IntoExpr,
    is_bid: IntoExpr,
    *,
    n: int = 1,
) -> pl.Expr:
    """
    Calculate the top `n` levels of the bid and ask sides of the order book from price mutations.

    This function processes price mutations such as additions and deletions.
    - An addition is represented by a positive quantity.
    - A deletion is represented by a negative quantity.

    Parameters
    ----------
    price : IntoExpr
        The price levels to be updated.
    qty : IntoExpr
        The corresponding quantities for each price level.
    is_bid : IntoExpr
        Boolean flag indicating whether the price level is on the bid side (True) or ask side (False).
    n : int, optional
        The number of top levels to calculate. Default is 1.

    Returns
    -------
    pl.Expr
        A Polars expression that computes the top `n` levels of the order book.

    Examples
    --------
    >>> import polars as pl
    >>> from polars_order_book import top_n_levels_from_price_mutations
    >>> df = pl.DataFrame({
    ...     "price": [100, 101, 102],
    ...     "qty": [10, 15, -5],
    ...     "is_bid": [True, True, False]
    ... })
    >>> expr = top_n_levels_from_price_mutations(df["price"], df["qty"], df["is_bid"], n=2)
    >>> df.with_columns(expr.alias("top_levels")).unnest("top_levels")
    """
    price = parse_into_expr(price)
    qty = parse_into_expr(qty)
    is_bid = parse_into_expr(is_bid)
    args = [price, qty, is_bid]

    return register_plugin(
        args=args,  # type: ignore
        symbol="pl_calculate_bbo_price_mutation",
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
    - A positive quantity sets or updates the quantity at the given price.
    - A zero quantity removes the price level.

    Parameters
    ----------
    price : IntoExpr
        The price levels to be updated.
    qty : IntoExpr
        The corresponding quantities for each price level.
    is_bid : IntoExpr
        Boolean flag indicating whether the price level is on the bid side (True) or ask side (False).
    n : int, optional
        The number of top levels to calculate. Default is 1.

    Returns
    -------
    pl.Expr
        A Polars expression that computes the top `n` levels of the order book.

    Examples
    --------
    >>> import polars as pl
    >>> from polars_order_book import top_n_levels_from_price_updates
    >>> df = pl.DataFrame({
    ...     "price": [10, 10, 10],
    ...     "qty": [100, 200, 0],
    ...     "is_bid": [True, True, True]
    ... })
    >>> expr = top_n_levels_from_price_updates(df["price"], df["qty"], df["is_bid"], n=2)
    >>> df.with_columns(expr.alias("top_levels")).unnest("top_levels")
    """
    price = parse_into_expr(price)
    qty = parse_into_expr(qty)
    is_bid = parse_into_expr(is_bid)
    args = [price, qty, is_bid]

    return register_plugin(
        args=args,  # type: ignore
        symbol="pl_calculate_bbo_price_update",
        is_elementwise=False,
        lib=lib,
        kwargs={"n": n},
    )
