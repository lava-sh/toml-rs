use lexical_core::ParseIntegerOptions;
use pyo3::{
    IntoPyObjectExt,
    exceptions::PyValueError,
    prelude::*,
    types::{PyDate, PyDelta, PyDict, PyList, PyTime, PyTzInfo},
};
use toml_v1::{Spanned, de::DeValue, value::Offset};

use crate::{
    create_py_datetime, error::TomlError, parse_int, recursion_guard::RecursionGuard,
    toml_rs::TOMLDecodeError,
};

pub(crate) fn toml_to_python_v1<'py>(
    py: Python<'py>,
    value: &Spanned<DeValue<'_>>,
    parse_float: &Bound<'py, PyAny>,
    doc: &str,
) -> PyResult<Bound<'py, PyAny>> {
    to_python(py, value, parse_float, &mut RecursionGuard::default(), doc)
}

fn to_python<'py>(
    py: Python<'py>,
    de_value: &Spanned<DeValue<'_>>,
    parse_float: &Bound<'py, PyAny>,
    recursion: &mut RecursionGuard,
    doc: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let value = de_value.get_ref();
    let span = de_value.span();

    match value {
        DeValue::String(str) => str.into_bound_py_any(py),
        DeValue::Integer(int) => {
            let bytes = int.as_str().as_bytes();
            let radix = int.radix();

            let options = ParseIntegerOptions::new();

            if let Ok(i_64) = parse_int!(i64, bytes, &options, radix) {
                return i_64.into_bound_py_any(py);
            }

            if let Ok(i_128) = parse_int!(i128, bytes, &options, radix) {
                return i_128.into_bound_py_any(py);
            }

            if let Some(bigint) = num_bigint::BigInt::parse_bytes(bytes, radix) {
                return bigint.into_bound_py_any(py);
            }

            let mut err = TomlError::custom(
                format!(
                    "invalid integer '{}'",
                    &doc[span.start..span.end.min(doc.len())]
                ),
                Some(span.start..span.end),
            );
            err.set_input(Some(doc));

            Err(TOMLDecodeError::new_err((
                err.to_string(),
                doc.to_string(),
                span.start,
            )))
        }
        DeValue::Float(float) => {
            let float_str = float.as_str();

            let py_call = parse_float.call1((float_str,))?;

            if py_call.is_exact_instance_of::<PyDict>() || py_call.is_exact_instance_of::<PyList>()
            {
                return Err(PyValueError::new_err(
                    "parse_float must not return dicts or lists",
                ));
            }

            Ok(py_call)
        }
        DeValue::Boolean(bool) => bool.into_bound_py_any(py),
        DeValue::Datetime(datetime) => match (datetime.date, datetime.time, datetime.offset) {
            (Some(date), Some(time), Some(offset)) => {
                let tzinfo = Some(&create_timezone_from_offset(py, offset)?);
                Ok(create_py_datetime!(py, date, time, tzinfo)?.into_any())
            }
            (Some(date), Some(time), None) => {
                Ok(create_py_datetime!(py, date, time, None)?.into_any())
            }
            (Some(date), None, None) => {
                let py_date = PyDate::new(py, i32::from(date.year), date.month, date.day)?;
                Ok(py_date.into_any())
            }
            (None, Some(time), None) => {
                let py_time = PyTime::new(
                    py,
                    time.hour,
                    time.minute,
                    time.second,
                    time.nanosecond / 1000,
                    None,
                )?;
                Ok(py_time.into_any())
            }
            _ => Err(PyValueError::new_err("Invalid datetime format")),
        },
        DeValue::Array(array) => {
            if array.is_empty() {
                return Ok(PyList::empty(py).into_any());
            }

            recursion.enter()?;
            let py_list = PyList::empty(py);
            for item in array {
                py_list.append(to_python(py, item, parse_float, recursion, doc)?)?;
            }
            recursion.exit();
            Ok(py_list.into_any())
        }
        DeValue::Table(table) => {
            if table.is_empty() {
                return Ok(PyDict::new(py).into_any());
            }

            recursion.enter()?;
            let py_dict = PyDict::new(py);
            for (k, v) in table {
                let key = k.get_ref().clone().into_owned();
                let value = to_python(py, v, parse_float, recursion, doc)?;
                py_dict.set_item(key, value)?;
            }
            recursion.exit();
            Ok(py_dict.into_any())
        }
    }
}

#[inline]
fn create_timezone_from_offset(py: Python, offset: Offset) -> PyResult<Bound<PyTzInfo>> {
    const SECS_IN_DAY: i32 = 86_400;

    match offset {
        Offset::Z => PyTzInfo::utc(py).map(Borrowed::to_owned),
        Offset::Custom { minutes } => {
            let seconds = i32::from(minutes) * 60;
            let days = seconds.div_euclid(SECS_IN_DAY);
            let seconds = seconds.rem_euclid(SECS_IN_DAY);
            let py_delta = PyDelta::new(py, days, seconds, 0, false)?;
            PyTzInfo::fixed_offset(py, py_delta)
        }
    }
}
