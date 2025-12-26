mod normalize;
mod recursion_guard;
mod v1_0_0;
mod v1_1_0;

use pyo3::{exceptions::PyValueError, import_exception, prelude::*};
use rustc_hash::FxHashSet;

use crate::{
    normalize::normalize_line_ending,
    v1_0_0::{
        dumps::{python_to_toml_v1_0_0, validate_inline_paths_v1_0_0},
        loads::toml_to_python_v1_0_0,
        pretty::PrettyV100,
    },
    v1_1_0::{
        dumps::{python_to_toml, validate_inline_paths},
        loads::toml_to_python,
        pretty::Pretty,
    },
};

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

import_exception!(toml_rs, TOMLDecodeError);
import_exception!(toml_rs, TOMLEncodeError);

#[pyfunction(name = "_loads")]
fn load_toml_from_string(
    py: Python,
    toml_string: &str,
    parse_float: Option<&Bound<'_, PyAny>>,
    toml_version: &str,
) -> PyResult<Py<PyAny>> {
    let normalized = normalize_line_ending(toml_string);

    match toml_version {
        "1.0.0" => {
            let parsed: toml_v1_0_0::Value = py
                .detach(|| toml_v1_0_0::from_str(&normalized))
                .map_err(|err| {
                    TOMLDecodeError::new_err((
                        err.to_string(),
                        normalized.to_string(),
                        err.span().map_or(0, |s| s.start),
                    ))
                })?;
            let toml = toml_to_python_v1_0_0(py, parsed, parse_float)?;
            Ok(toml.unbind())
        }
        "1.1.0" => {
            use toml::de::{DeTable, DeValue::Table};

            let parsed = DeTable::parse(&normalized).map_err(|err| {
                TOMLDecodeError::new_err((
                    err.to_string(),
                    normalized.to_string(),
                    err.span().map_or(0, |s| s.start),
                ))
            })?;
            let toml = toml_to_python(py, Table(parsed.into_inner()), parse_float)?;
            Ok(toml.unbind())
        }
        _ => Err(PyValueError::new_err(format!(
            "Unsupported TOML version: {toml_version}. Supported versions: 1.0.0, 1.1.0",
        ))),
    }
}

#[allow(clippy::needless_pass_by_value)]
#[pyfunction(name = "_dumps")]
fn dumps_toml(
    py: Python,
    obj: &Bound<'_, PyAny>,
    pretty: bool,
    inline_tables: Option<FxHashSet<String>>,
    toml_version: &str,
) -> PyResult<String> {
    match toml_version {
        "1.0.0" => {
            use toml_edit_v1_0_0::{DocumentMut, Item::Table, visit_mut::VisitMut};

            let mut doc = DocumentMut::new();

            if let Table(table) = python_to_toml_v1_0_0(py, obj, inline_tables.as_ref())? {
                *doc.as_table_mut() = table;
            }

            if let Some(ref paths) = inline_tables {
                validate_inline_paths_v1_0_0(doc.as_item(), paths)?;
            }

            if pretty {
                PrettyV100::new(inline_tables.is_none()).visit_document_mut(&mut doc);
            }

            Ok(doc.to_string())
        }
        "1.1.0" => {
            use toml_edit::{DocumentMut, Item::Table, visit_mut::VisitMut};

            let mut doc = DocumentMut::new();

            if let Table(table) = python_to_toml(py, obj, inline_tables.as_ref())? {
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
        _ => Err(PyValueError::new_err(format!(
            "Unsupported TOML version: {toml_version}. Supported versions: 1.0.0, 1.1.0",
        ))),
    }
}

#[pymodule(name = "_toml_rs")]
fn toml_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(load_toml_from_string, m)?)?;
    m.add_function(wrap_pyfunction!(dumps_toml, m)?)?;
    m.add("_version", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
