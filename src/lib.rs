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

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

import_exception!(toml_rs, TOMLDecodeError);
import_exception!(toml_rs, TOMLEncodeError);

#[pyfunction(name = "_loads")]
fn load_toml_from_string(
    py: Python,
    s: &str,
    parse_float: Option<&Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let normalized = normalize_line_ending(s);
    let value = py.detach(|| toml::from_str(&normalized)).map_err(|err| {
        TOMLDecodeError::new_err((
            err.to_string(),
            normalized.to_string(),
            err.span().map_or(0, |s| s.start),
        ))
    })?;
    let toml = toml_to_python(py, value, parse_float)?;
    Ok(toml.unbind())
}

#[allow(clippy::needless_pass_by_value)]
#[pyfunction(name = "_dumps")]
fn dumps_toml(
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

    if pretty {
        Pretty::new(inline_tables.is_none()).visit_document_mut(&mut doc);
    }

    Ok(doc.to_string())
}

#[pymodule(name = "_toml_rs")]
fn toml_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(load_toml_from_string, m)?)?;
    m.add_function(wrap_pyfunction!(dumps_toml, m)?)?;
    m.add("_version", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
