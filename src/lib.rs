mod conversion;
mod macros;

use crate::conversion::{normalize_line_ending, python_to_toml, toml_to_python};

use pyo3::{exceptions::PyTypeError, import_exception, prelude::*, types::PyBytes};

#[cfg(feature = "default")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

import_exception!(toml_rs, TOMLDecodeError);
import_exception!(toml_rs, TOMLEncodeError);

#[pyfunction(name = "_loads")]
fn loads(py: Python, s: &str, parse_float: Option<Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
    let normalized = normalize_line_ending(s);
    let value = py.detach(|| toml::from_str(&normalized)).map_err(|err| {
        TOMLDecodeError::new_err((
            err.to_string(),
            normalized.to_string(),
            err.span().map(|s| s.start).unwrap_or(0),
        ))
    })?;
    let result = toml_to_python(py, value, parse_float.as_ref())?;
    Ok(result.unbind())
}

#[pyfunction(name = "_load")]
fn load(py: Python, fp: Py<PyAny>, parse_float: Option<Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
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

    loads(py, &s, parse_float)
}

#[pyfunction(name = "_dumps")]
fn dumps(py: Python, obj: &Bound<'_, PyAny>, pretty: bool) -> PyResult<String> {
    let value = python_to_toml(py, obj)?;
    let toml = if pretty {
        toml::to_string_pretty(&value)
    } else {
        toml::to_string(&value)
    }
    .map_err(|err| TOMLEncodeError::new_err(err.to_string()))?;
    Ok(toml)
}

#[pymodule(name = "_toml_rs")]
fn toml_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(load, m)?)?;
    m.add_function(wrap_pyfunction!(loads, m)?)?;
    m.add_function(wrap_pyfunction!(dumps, m)?)?;
    m.add("_version", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
