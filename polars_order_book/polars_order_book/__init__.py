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


def calculate_bbo(
    price: IntoExpr,
    qty: IntoExpr,
    is_bid: IntoExpr,
    prev_price: IntoExpr | None = None,
    prev_qty: IntoExpr | None = None,
    *,
    n: int = 1,
) -> pl.Expr:
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
