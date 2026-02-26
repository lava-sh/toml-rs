use pyo3::prelude::*;
use toml::{Spanned, de::DeValue};

use crate::{create_py_datetime, impl_loads};

pub(crate) fn toml_to_python<'py>(
    py: Python<'py>,
    value: &Spanned<DeValue<'_>>,
    parse_float: &Bound<'py, PyAny>,
    doc: &str,
) -> PyResult<Bound<'py, PyAny>> {
    to_python(py, value, parse_float, doc)
}

impl_loads!(
    create_py_datetime,
    |time: &toml::value::Time| time.second.unwrap_or(0),
    |time: &toml::value::Time| time.nanosecond.unwrap_or(0) / 1000
);
