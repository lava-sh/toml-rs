mod dumps;
mod loads;
mod macros;
mod recursion_guard;

use crate::{
    dumps::python_to_toml,
    loads::{normalize_line_ending, toml_to_python},
};

use rustc_hash::FxHashSet;
use pyo3::{import_exception, prelude::*};

#[cfg(feature = "default")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

import_exception!(toml_rs, TOMLDecodeError);
import_exception!(toml_rs, TOMLEncodeError);

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
    let toml = toml_to_python(py, value, parse_float.as_ref())?;
    Ok(toml.unbind())
}

#[pyfunction]
fn _dumps(
    py: Python,
    obj: &Bound<'_, PyAny>,
    pretty: bool,
    inline_tables: Option<FxHashSet<String>>,
) -> PyResult<String> {
    let to_toml = python_to_toml(py, obj, inline_tables.as_ref())?;

    let mut toml = toml_edit::DocumentMut::new();
    if let toml_edit::Item::Table(table) = to_toml {
        *toml.as_table_mut() = table;
    }

    if let Some(paths) = inline_tables {
        for path in paths {
            let mut current = toml.as_item();

            for key in path.split(".") {
                if let Some(item) = current.get(key) {
                    current = item;
                } else {
                    return Err(TOMLEncodeError::new_err(format!(
                        "Path '{}' specified in inline_tables does not exist in the toml",
                        path
                    )));
                }
            }
            if !current.is_table() && !current.is_inline_table() {
                return Err(TOMLEncodeError::new_err(format!(
                    "Path '{}' does not point to a table",
                    path
                )));
            }
        }
    }
    if pretty {
        toml.fmt();
    }
    Ok(toml.to_string())
}

#[pymodule(name = "_toml_rs")]
fn toml_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(_loads, m)?)?;
    m.add_function(wrap_pyfunction!(_dumps, m)?)?;
    m.add("_version", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
