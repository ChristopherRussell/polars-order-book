import polars as pl
import pytest
from polars.testing.asserts import assert_frame_equal

from polars_order_book import calculate_bbo


@pytest.mark.parametrize("n", [1, 10, 100])
def test_calculate_bbo(n: int):
    n = 10
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
        bbo=calculate_bbo("price", "qty", "is_bid")
    ).unnest("bbo")

    expected_values = {
        "id": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        "best_bid": [1, 2, 3, 3, 3, 3, 2, 2, None, None, None, None],
        "best_ask": [None, None, None, 6, 5, 4, 4, 4, 4, 4, 6, None],
        "best_bid_qty": [1, 2, 3, 3, 3, 3, 2, 2, None, None, None, None],
        "best_ask_qty": [None, None, None, 6, 5, 4, 4, 4, 4, 4, 6, None],
    }
    expected = pl.DataFrame(
        expected_values,
        schema={k: v for k, v in market_data.schema.items() if k in expected_values},
    )
    expected = market_data.select("id").join(expected, on="id")

    assert_frame_equal(
        market_data.select(
            "id", "best_bid", "best_ask", "best_bid_qty", "best_ask_qty"
        ),
        expected,
        check_column_order=False,
    )
