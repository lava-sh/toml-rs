mod conversion;
mod macros;

use crate::conversion::{normalize_line_ending, python_to_toml, toml_to_python};

use pyo3::{import_exception, prelude::*};

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
    m.add_function(wrap_pyfunction!(loads, m)?)?;
    m.add_function(wrap_pyfunction!(dumps, m)?)?;
    m.add("_version", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
