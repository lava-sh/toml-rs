use pyo3::{
    IntoPyObjectExt,
    exceptions::PyValueError,
    prelude::*,
    types::{PyDate, PyDelta, PyDict, PyList, PyTime, PyTzInfo},
};
use toml_v1_0_0::{Value, value::Offset};

use crate::{create_py_datetime, recursion_guard::RecursionGuard};

pub(crate) fn toml_to_python_v1_0_0<'py>(
    py: Python<'py>,
    value: Value,
    parse_float: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    to_python(py, value, parse_float, &mut RecursionGuard::default())
}

#[inline]
fn to_python<'py>(
    py: Python<'py>,
    value: Value,
    parse_float: Option<&Bound<'py, PyAny>>,
    recursion: &mut RecursionGuard,
) -> PyResult<Bound<'py, PyAny>> {
    match value {
        Value::String(str) => str.into_bound_py_any(py),
        Value::Integer(int) => int.into_bound_py_any(py),
        Value::Float(float) => {
            let Some(f) = parse_float else {
                return float.into_bound_py_any(py);
            };

            let mut buffer = zmij::Buffer::new();
            let formatted = buffer.format(float);

            let py_call = f.call1((formatted,))?;

            if py_call.is_exact_instance_of::<PyDict>() || py_call.is_exact_instance_of::<PyList>()
            {
                return Err(PyValueError::new_err(
                    "parse_float must not return dicts or lists",
                ));
            }

            Ok(py_call)
        }
        Value::Boolean(bool) => bool.into_bound_py_any(py),
        Value::Datetime(datetime) => match (datetime.date, datetime.time, datetime.offset) {
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
        Value::Array(array) => {
            if array.is_empty() {
                return Ok(PyList::empty(py).into_any());
            }

            recursion.enter()?;
            let py_list = PyList::empty(py);
            for item in array {
                py_list.append(to_python(py, item, parse_float, recursion)?)?;
            }
            recursion.exit();
            Ok(py_list.into_any())
        }
        Value::Table(table) => {
            if table.is_empty() {
                return Ok(PyDict::new(py).into_any());
            }

            recursion.enter()?;
            let py_dict = PyDict::new(py);
            for (k, v) in table {
                let value = to_python(py, v, parse_float, recursion)?;
                py_dict.set_item(k, value)?;
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
