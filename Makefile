SHELL=/bin/bash

.venv:  ## Set up virtual environment
	python3 -m venv polars_order_book/.venv
	polars_order_book/.venv/bin/pip install -r polars_order_book/requirements.txt

install: .venv
	unset CONDA_PREFIX && \
	source polars_order_book/.venv/bin/activate && maturin develop --manifest-path polars_order_book/Cargo.toml

install-release: .venv
	unset CONDA_PREFIX && \
	source polars_order_book/.venv/bin/activate && maturin develop --release --manifest-path polars_order_book/Cargo.toml

pre-commit: .venv
	cargo fmt --all && cargo clippy --all-features
	polars_order_book/.venv/bin/python -m ruff check polars_order_book --fix --exit-non-zero-on-fix
	polars_order_book/.venv/bin/python -m ruff format polars_order_book tests
	polars_order_book/.venv/bin/mypy polars_order_book tests

test: .venv
	polars_order_book/.venv/bin/python -m pytest polars_order_book/tests

run: install
	source polars_order_book/.venv/bin/activate && python polars_order_book/run.py

run-release: install-release
	source polars_order_book/.venv/bin/activate && python polars_order_book/run.py
