from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import polars as pl

from polars_order_book.utils import parse_into_expr, parse_version, register_plugin

if TYPE_CHECKING:
    from polars.type_aliases import IntoExpr

if parse_version(pl.__version__) < parse_version("0.20.16"):
    from polars.utils.udfs import _get_shared_lib_location

    lib: str | Path = _get_shared_lib_location(__file__)
else:
    lib = Path(__file__).parent


def calculate_bbo(price: IntoExpr, qty: IntoExpr, is_bid: IntoExpr) -> pl.Expr:
    price = parse_into_expr(price)
    qty = parse_into_expr(qty)
    is_bid = parse_into_expr(is_bid)
    return register_plugin(
        args=[price, qty, is_bid],
        symbol="pl_calculate_bbo",
        is_elementwise=False,
        lib=lib,
    )
