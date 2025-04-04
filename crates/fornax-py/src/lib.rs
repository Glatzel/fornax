use pyo3::prelude::*;
#[pymodule]
fn fornax_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // m.add_wrapped(wrap_pyfunction!(crypto::py_crypto))?;

    Ok(())
}
