use pyo3::prelude::*;
use toml_v1::{Spanned, de::DeValue};

use crate::{create_py_datetime_v1, impl_to_python};

pub(crate) fn toml_to_python_v1<'py>(
    py: Python<'py>,
    value: &Spanned<DeValue<'_>>,
    parse_float: &Bound<'py, PyAny>,
    doc: &str,
) -> PyResult<Bound<'py, PyAny>> {
    to_python(py, value, parse_float, doc)
}

impl_to_python!(
    create_py_datetime_v1,
    |time: &toml_v1::value::Time| time.second,
    |time: &toml_v1::value::Time| time.nanosecond / 1000
);
