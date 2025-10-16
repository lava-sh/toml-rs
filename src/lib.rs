mod conversion;

use crate::conversion::{convert_toml, normalize_line_ending};

use pyo3::{exceptions::PyTypeError, prelude::*, types::PyBytes};

#[cfg(not(any(
    all(target_os = "linux", target_arch = "aarch64"),
    all(target_os = "linux", target_arch = "arm"),
    all(target_os = "linux", target_arch = "s390x"),
    all(target_os = "linux", target_arch = "powerpc64")
)))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pyo3::import_exception!(toml_rs, TOMLDecodeError);

#[pyfunction]
fn _loads(py: Python, s: &str, parse_float: Option<Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
    let normalized = normalize_line_ending(s);
    let value = py.detach(|| toml::from_str(&normalized)).map_err(|err| {
        TOMLDecodeError::new_err((
            err.to_string(),
            normalized.to_string(),
            err.span().map(|s| s.start).unwrap_or(0),
        ))
    })?;
    let result = convert_toml(py, value, parse_float.as_ref())?;
    Ok(result.unbind())
}

#[pyfunction]
fn _load(py: Python, fp: Py<PyAny>, parse_float: Option<Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
    let bound = fp.bind(py);
    let read = bound.getattr("read")?;
    let content_obj = read.call0()?;

    let s = if let Ok(bytes) = content_obj.cast::<PyBytes>() {
        match std::str::from_utf8(bytes.as_bytes()) {
            Ok(valid) => valid.to_string(),
            Err(_) => String::from_utf8_lossy(bytes.as_bytes()).into_owned(),
        }
    } else if content_obj.extract::<&str>().is_ok() {
        return Err(PyErr::new::<PyTypeError, _>(
            "File must be opened in binary mode, e.g. use `open('foo.toml', 'rb')`",
        ));
    } else {
        return Err(PyErr::new::<PyTypeError, _>(
            "Expected bytes-like object from .read()",
        ));
    };

    _loads(py, &s, parse_float)
}

#[pymodule]
fn _toml_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(_load, m)?)?;
    m.add_function(wrap_pyfunction!(_loads, m)?)?;
    m.add("_version", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
