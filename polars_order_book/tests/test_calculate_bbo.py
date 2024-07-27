import polars as pl
import pytest
from polars.testing.asserts import assert_frame_equal

from polars_order_book import calculate_bbo


@pytest.mark.parametrize("nr_tracked_levels", [1, 2, 4])
@pytest.mark.parametrize("n", [1, 10, 100, 1000])
def test_calculate_bbo(n: int, nr_tracked_levels: int):
    market_data = pl.DataFrame(
        {
            "id": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] * n,
            "price": [1, 2, 3, 6, 5, 4, 3, 1, 2, 5, 4, 6] * n,
            "qty": [1, 2, 3, 6, 5, 4, -3, -1, -2, -5, -4, -6] * n,
            "is_bid": [
                True,
                True,
                True,
                False,
                False,
                False,
                True,
                True,
                True,
                False,
                False,
                False,
            ]
            * n,
        },
        schema={
            "id": pl.Int8,
            "price": pl.Int64,
            "qty": pl.Int64,
            "is_bid": pl.Boolean,
        },
    )
    market_data = market_data.with_columns(
        bbo=calculate_bbo("price", "qty", "is_bid", n=nr_tracked_levels)
    ).unnest("bbo")

    expected_values = {
        "id": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        "bid_price_1": [1, 2, 3, 3, 3, 3, 2, 2, None, None, None, None],
        "ask_price_1": [None, None, None, 6, 5, 4, 4, 4, 4, 4, 6, None],
        "bid_qty_1": [1, 2, 3, 3, 3, 3, 2, 2, None, None, None, None],
        "ask_qty_1": [None, None, None, 6, 5, 4, 4, 4, 4, 4, 6, None],
    }
    expected = pl.DataFrame(
        expected_values,
        schema={k: v for k, v in market_data.schema.items() if k in expected_values},
    )
    expected = market_data.select("id").join(expected, on="id")

    assert_frame_equal(
        market_data.select(
            "id", "bid_price_1", "ask_price_1", "bid_qty_1", "ask_qty_1"
        ),
        expected,
        check_column_order=False,
    )


@pytest.mark.parametrize("nr_tracked_levels", [1, 2, 4])
@pytest.mark.parametrize("n", [1, 10, 100, 1000])
def test_calculate_bbo_with_mods(n: int, nr_tracked_levels: int):
    market_data = pl.DataFrame(
        {
            "id": [-2, -1] + [1, 2, 3, 4, 5, 6] * n,
            "price": [1, 6] + [2, 3, 1, 5, 4, 6] * n,
            "qty": [1, 6] + [2, 3, 1, 5, 4, 6] * n,
            "is_bid": [True, False]
            + [
                True,
                True,
                True,
                False,
                False,
                False,
            ]
            * n,
            "prev_price": [None, None] + [1, 2, 3, 6, 5, 4] * n,
            "prev_qty": [None, None] + [1, 2, 3, 6, 5, 4] * n,
        },
        schema={
            "id": pl.Int8,
            "price": pl.Int64,
            "qty": pl.Int64,
            "is_bid": pl.Boolean,
            "prev_price": pl.Int64,
            "prev_qty": pl.Int64,
        },
    )

    market_data = market_data.with_columns(
        bbo=calculate_bbo(
            "price", "qty", "is_bid", "prev_price", "prev_qty", n=nr_tracked_levels
        )
    ).unnest("bbo")

    expected_values = {
        "id": [-2, -1, 1, 2, 3, 4, 5, 6],
        "bid_price_1": [1, 1, 2, 3, 1, 1, 1, 1],
        "ask_price_1": [None, 6, 6, 6, 6, 5, 4, 6],
        "bid_qty_1": [1, 1, 2, 3, 1, 1, 1, 1],
        "ask_qty_1": [None, 6, 6, 6, 6, 5, 4, 6],
    }
    expected = pl.DataFrame(
        expected_values,
        schema={k: v for k, v in market_data.schema.items() if k in expected_values},
    )
    expected = market_data.select("id").join(expected, on="id")

    assert_frame_equal(
        market_data.select(
            "id", "bid_price_1", "ask_price_1", "bid_qty_1", "ask_qty_1"
        ),
        expected,
        check_column_order=False,
    )


@pytest.mark.parametrize("nr_tracked_levels", [1, 2, 4])
@pytest.mark.parametrize("n", [1, 10, 100, 1000])
def test_calculate_bbo_with_modifies_not_used(n: int, nr_tracked_levels: int):
    market_data = pl.DataFrame(
        {
            "id": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12] * n,
            "price": [1, 2, 3, 6, 5, 4, 3, 1, 2, 5, 4, 6] * n,
            "qty": [1, 2, 3, 6, 5, 4, -3, -1, -2, -5, -4, -6] * n,
            "is_bid": [
                True,
                True,
                True,
                False,
                False,
                False,
                True,
                True,
                True,
                False,
                False,
                False,
            ]
            * n,
        },
        schema={
            "id": pl.Int8,
            "price": pl.Int64,
            "qty": pl.Int64,
            "is_bid": pl.Boolean,
        },
    ).with_columns(
        prev_price=pl.lit(None, dtype=pl.Int64), prev_qty=pl.lit(None, dtype=pl.Int64)
    )
    market_data = market_data.with_columns(
        bbo=calculate_bbo(
            "price", "qty", "is_bid", "prev_price", "prev_qty", n=nr_tracked_levels
        )
    ).unnest("bbo")

    expected_values = {
        "id": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        "bid_price_1": [1, 2, 3, 3, 3, 3, 2, 2, None, None, None, None],
        "ask_price_1": [None, None, None, 6, 5, 4, 4, 4, 4, 4, 6, None],
        "bid_qty_1": [1, 2, 3, 3, 3, 3, 2, 2, None, None, None, None],
        "ask_qty_1": [None, None, None, 6, 5, 4, 4, 4, 4, 4, 6, None],
    }
    expected = pl.DataFrame(
        expected_values,
        schema={k: v for k, v in market_data.schema.items() if k in expected_values},
    )
    expected = market_data.select("id").join(expected, on="id")

    assert_frame_equal(
        market_data.select(
            "id", "bid_price_1", "ask_price_1", "bid_qty_1", "ask_qty_1"
        ),
        expected,
        check_column_order=False,
    )


@pytest.mark.parametrize("nr_tracked_levels", [1, 2, 4])
@pytest.mark.parametrize("n, is_bid", [(1, True), (1, False), (10, True), (10, False)])
def test_multiple_orders_per_level_modify(n: int, is_bid: bool, nr_tracked_levels: int):
    market_data = pl.DataFrame(
        {
            "id": [1, 2, 3, 4] + [5, 6, 3, 4] * n,
            "price": [1, 1, 2, 2] + [1, 1, 2, 2] * n,
            "qty": [1, 1, 2, 2] + [1, 1, 2, 2] * n,
            "prev_price": [None, None, 1, 1] + [2, 2, 1, 1] * n,
            "prev_qty": [None, None, 1, 1] + [2, 2, 1, 1] * n,
        },
        schema={
            "id": pl.Int8,
            "price": pl.Int64,
            "qty": pl.Int64,
            "prev_price": pl.Int64,
            "prev_qty": pl.Int64,
        },
    ).with_columns(is_bid=is_bid)

    market_data = market_data.with_columns(
        bbo=calculate_bbo(
            "price", "qty", "is_bid", "prev_price", "prev_qty", n=nr_tracked_levels
        )
    ).unnest("bbo")

    if is_bid:
        side, other_side = "bid", "ask"
        best_px = [1, 1, 2, 2, 2, 1]
        best_px_qty = [1, 2, 2, 4, 2, 2]
    else:
        side, other_side = "ask", "bid"
        best_px = [1, 1, 1, 2, 1, 1]
        best_px_qty = [1, 2, 1, 4, 1, 2]
    expected_values = {
        "id": [1, 2, 3, 4, 5, 6],
        f"{side}_price_1": best_px,
        f"{other_side}_price_1": [None] * 6,
        f"{side}_qty_1": best_px_qty,
        f"{other_side}_qty_1": [None] * 6,
    }
    expected = pl.DataFrame(
        expected_values,
        schema={k: v for k, v in market_data.schema.items() if k in expected_values},
    )
    expected = market_data.select("id").join(expected, on="id")

    assert_frame_equal(
        market_data.select(
            "id", "bid_price_1", "ask_price_1", "bid_qty_1", "ask_qty_1"
        ),
        expected,
        check_column_order=False,
    )
