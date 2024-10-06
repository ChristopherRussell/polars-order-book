mod calculate_bbo;
mod errors;
mod output;
mod update;
mod utils;

use pyo3::types::PyModule;
use pyo3::{pymodule, types::PyModuleMethods, Bound, PyResult, Python};
use pyo3_polars::PolarsAllocator;

#[pymodule]
fn _internal(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}

#[global_allocator]
static ALLOC: PolarsAllocator = PolarsAllocator::new();
