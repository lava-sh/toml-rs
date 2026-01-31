mod normalize;
mod recursion_guard;
mod v1;
mod v1_1;

#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[pyo3::pymodule(name = "_toml_rs")]
mod toml_rs {
    use pyo3::{exceptions::PyValueError, import_exception, prelude::*};
    use rustc_hash::FxHashSet;

    use crate::{
        normalize::normalize_line_ending,
        v1::{
            dumps::{python_to_toml_v1, validate_inline_paths_v1},
            loads::toml_to_python_v1,
            pretty::PrettyV100,
        },
        v1_1::{
            dumps::{python_to_toml, validate_inline_paths},
            loads::toml_to_python,
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

                let normalized = normalize_line_ending(toml_string);

                let parsed = DeTable::parse(&normalized).map_err(|err| {
                    TOMLDecodeError::new_err((
                        err.to_string(),
                        normalized.to_string(),
                        err.span().map_or(0, |s| s.start),
                    ))
                })?;

                let toml = toml_to_python_v1(
                    py,
                    &Spanned::new(parsed.span(), DeValue::Table(parsed.into_inner())),
                    parse_float,
                    &normalized,
                )?;

                Ok(toml.unbind())
            }
            "1.1.0" => {
                use toml::{
                    Spanned,
                    de::{DeTable, DeValue},
                };

                let normalized = normalize_line_ending(toml_string);

                let parsed = DeTable::parse(&normalized).map_err(|err| {
                    TOMLDecodeError::new_err((
                        err.to_string(),
                        normalized.to_string(),
                        err.span().map_or(0, |s| s.start),
                    ))
                })?;

                let toml = toml_to_python(
                    py,
                    &Spanned::new(parsed.span(), DeValue::Table(parsed.into_inner())),
                    parse_float,
                    &normalized,
                )?;

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
                use toml_edit_v1::{DocumentMut, Item::Table, visit_mut::VisitMut};

                let mut doc = DocumentMut::new();

                if let Table(table) = python_to_toml_v1(py, obj, inline_tables.as_ref())? {
                    *doc.as_table_mut() = table;
                }

                if let Some(ref paths) = inline_tables {
                    validate_inline_paths_v1(doc.as_item(), paths)?;
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
}
