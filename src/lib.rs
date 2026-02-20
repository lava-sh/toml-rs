mod document;
mod error;
mod v1;
mod v1_1;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL_ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(feature = "snmalloc")]
#[global_allocator]
static GLOBAL_ALLOCATOR: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

#[pyo3::pymodule(name = "_toml_rs")]
mod toml_rs {
    use pyo3::{exceptions::PyValueError, import_exception, prelude::*};
    use rustc_hash::FxHashSet;

    #[pymodule_export]
    use crate::document::TOMLDocument;
    use crate::{
        v1::{
            dumps::{python_to_toml_v1, validate_inline_paths_v1},
            loads::toml_to_python_v1,
            metadata::{extract_metadata_v1, to_python_v1},
            pretty::PrettyV1,
        },
        v1_1::{
            dumps::{python_to_toml, validate_inline_paths},
            loads::toml_to_python,
            metadata::{extract_metadata, to_python},
            pretty::Pretty,
        },
    };
    #[pymodule_export]
    const _VERSION: &str = env!("CARGO_PKG_VERSION");

    import_exception!(toml_rs, TOMLDecodeError);
    import_exception!(toml_rs, TOMLEncodeError);

    #[pyfunction(name = "_loads")]
    fn load_toml_from_string(
        py: Python,
        toml_string: &str,
        parse_float: &Bound<'_, PyAny>,
        toml_version: &str,
    ) -> PyResult<Py<PyAny>> {
        match toml_version {
            "1.0.0" => {
                use toml_v1::{
                    Spanned,
                    de::{DeTable, DeValue},
                };

                let parsed = DeTable::parse(toml_string).map_err(|err| {
                    TOMLDecodeError::new_err((
                        err.to_string(),
                        toml_string.to_string(),
                        err.span().map_or(0, |s| s.start),
                    ))
                })?;

                let toml = toml_to_python_v1(
                    py,
                    &Spanned::new(parsed.span(), DeValue::Table(parsed.into_inner())),
                    parse_float,
                    toml_string,
                )?;

                Ok(toml.unbind())
            }
            "1.1.0" => {
                use toml::{
                    Spanned,
                    de::{DeTable, DeValue},
                };

                let parsed = DeTable::parse(toml_string).map_err(|err| {
                    TOMLDecodeError::new_err((
                        err.to_string(),
                        toml_string.to_string(),
                        err.span().map_or(0, |s| s.start),
                    ))
                })?;

                let toml = toml_to_python(
                    py,
                    &Spanned::new(parsed.span(), DeValue::Table(parsed.into_inner())),
                    parse_float,
                    toml_string,
                )?;

                Ok(toml.unbind())
            }
            _ => Err(PyValueError::new_err(format!(
                "Unsupported TOML version: {toml_version}",
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
                use toml_edit_v1::{DocumentMut, Item::Table, visit_mut::VisitMut};

                let mut doc = DocumentMut::new();

                if let Table(table) = python_to_toml_v1(py, obj, inline_tables.as_ref())? {
                    *doc.as_table_mut() = table;
                }

                if let Some(ref paths) = inline_tables {
                    validate_inline_paths_v1(doc.as_item(), paths)?;
                }

                if pretty {
                    PrettyV1::new(inline_tables.is_none()).visit_document_mut(&mut doc);
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
                "Unsupported TOML version: {toml_version}",
            ))),
        }
    }

    #[pyfunction(name = "_parse_from_string")]
    fn parse_toml_from_string(
        py: Python,
        toml_string: &str,
        toml_version: &str,
    ) -> PyResult<Py<PyAny>> {
        match toml_version {
            "1.0.0" => {
                use toml_v1::de::{DeTable, DeValue};

                let parsed = DeTable::parse(toml_string).map_err(|err| {
                    TOMLDecodeError::new_err((
                        err.to_string(),
                        toml_string.to_string(),
                        err.span().map_or(0, |s| s.start),
                    ))
                })?;

                let meta = extract_metadata_v1(py, &parsed, toml_string)?;

                let span = parsed.span();
                let inner = parsed.into_inner();
                let value = to_python_v1(py, &DeValue::Table(inner), span, toml_string)?;

                let doc = Py::new(
                    py,
                    TOMLDocument {
                        value: value.unbind(),
                        meta: meta.unbind(),
                    },
                )?;

                Ok(doc.into())
            }
            "1.1.0" => {
                use toml::de::{DeTable, DeValue};

                let parsed = DeTable::parse(toml_string).map_err(|err| {
                    TOMLDecodeError::new_err((
                        err.to_string(),
                        toml_string.to_string(),
                        err.span().map_or(0, |s| s.start),
                    ))
                })?;

                let meta = extract_metadata(py, &parsed, toml_string)?;

                let span = parsed.span();
                let inner = parsed.into_inner();
                let value = to_python(py, &DeValue::Table(inner), span, toml_string)?;

                let doc = Py::new(
                    py,
                    TOMLDocument {
                        value: value.unbind(),
                        meta: meta.unbind(),
                    },
                )?;

                Ok(doc.into())
            }
            _ => Err(PyValueError::new_err(format!(
                "Unsupported TOML version: {toml_version}",
            ))),
        }
    }
}
