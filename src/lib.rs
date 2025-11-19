mod dumps;
mod loads;
mod macros;
mod pretty;
mod recursion_guard;

use pyo3::{import_exception, prelude::*};
use rustc_hash::FxHashSet;
use toml_edit::{DocumentMut, Item, visit_mut::VisitMut};

use crate::{
    dumps::{python_to_toml, validate_inline_paths},
    loads::{normalize_line_ending, toml_to_python},
    pretty::Pretty,
};

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
    let mut doc = DocumentMut::new();

    if let Item::Table(table) = python_to_toml(py, obj, inline_tables.as_ref())? {
        *doc.as_table_mut() = table;
    }

    if let Some(ref paths) = inline_tables {
        validate_inline_paths(doc.as_item(), paths)?;
    }

    let toml = if pretty {
        Pretty::new(inline_tables.is_none()).visit_document_mut(&mut doc);
        doc.to_string()
    } else {
        doc.to_string()
    };

    Ok(toml)
}

#[pymodule(name = "_toml_rs")]
fn toml_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(_loads, m)?)?;
    m.add_function(wrap_pyfunction!(_dumps, m)?)?;
    m.add("_version", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
