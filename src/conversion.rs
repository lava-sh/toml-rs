use std::borrow::Cow;

use pyo3::{
    IntoPyObjectExt,
    exceptions::{PyRecursionError, PyValueError},
    prelude::*,
    types::{PyDate, PyDateTime, PyDelta, PyDict, PyList, PyTime, PyTzInfo},
};
use toml_datetime::Offset;

const MAX_RECURSION_DEPTH: usize = 999;

#[derive(Clone, Debug, Default)]
struct RecursionCheck {
    current: usize,
}

impl RecursionCheck {
    fn enter(&mut self) -> PyResult<()> {
        self.current += 1;
        if MAX_RECURSION_DEPTH <= self.current {
            return Err(PyRecursionError::new_err(
                "max recursion depth met".to_string(),
            ));
        }
        Ok(())
    }

    fn exit(&mut self) {
        self.current -= 1;
    }
}

pub(crate) fn convert_toml<'py>(
    py: Python<'py>,
    value: toml::Value,
    parse_float: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut recursion_check = RecursionCheck::default();
    _convert_toml(py, value, parse_float, &mut recursion_check)
}

fn _convert_toml<'py>(
    py: Python<'py>,
    value: toml::Value,
    parse_float: Option<&Bound<'py, PyAny>>,
    recursion_check: &mut RecursionCheck,
) -> PyResult<Bound<'py, PyAny>> {
    recursion_check.enter()?;

    let toml = match value {
        toml::Value::String(str) => str.into_bound_py_any(py),
        toml::Value::Integer(int) => int.into_bound_py_any(py),
        toml::Value::Float(float) => {
            if let Some(parse_float) = parse_float {
                let result = parse_float.call1((float.to_string(),))?;
                if result.is_instance_of::<PyDict>() || result.is_instance_of::<PyList>() {
                    return Err(PyValueError::new_err(
                        "parse_float must not return dicts or lists",
                    ));
                }
                Ok(result)
            } else {
                float.into_bound_py_any(py)
            }
        }
        toml::Value::Boolean(bool) => bool.into_bound_py_any(py),
        toml::Value::Array(array) => {
            let mut values = Vec::with_capacity(array.len());
            for item in array {
                values.push(_convert_toml(py, item, parse_float, recursion_check)?);
            }
            Ok(PyList::new(py, values)?.into_any())
        }
        toml::Value::Table(table) => {
            let dict = PyDict::new(py);
            for (k, v) in table {
                let value = _convert_toml(py, v, parse_float, recursion_check)?;
                dict.set_item(k, value)?;
            }
            Ok(dict.into_any())
        }
        toml::Value::Datetime(datetime) => match (datetime.date, datetime.time, datetime.offset) {
            (Some(date), Some(time), Some(offset)) => {
                let py_datetime = PyDateTime::new(
                    py,
                    date.year as i32,
                    date.month,
                    date.day,
                    time.hour,
                    time.minute,
                    time.second,
                    time.nanosecond / 1000,
                    Some(&create_timezone_from_offset(py, &offset)?),
                )?;
                Ok(py_datetime.into_any())
            }
            (Some(date), Some(time), None) => {
                let py_datetime = PyDateTime::new(
                    py,
                    date.year as i32,
                    date.month,
                    date.day,
                    time.hour,
                    time.minute,
                    time.second,
                    time.nanosecond / 1000,
                    None,
                )?;
                Ok(py_datetime.into_any())
            }
            (Some(date), None, None) => {
                let py_date = PyDate::new(py, date.year as i32, date.month, date.day)?;
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
    };
    recursion_check.exit();
    toml
}

fn create_timezone_from_offset<'py>(
    py: Python<'py>,
    offset: &Offset,
) -> PyResult<Bound<'py, PyTzInfo>> {
    match offset {
        Offset::Z => PyTzInfo::utc(py).map(|utc| utc.to_owned()),
        Offset::Custom { minutes } => {
            let seconds = *minutes as i32 * 60;
            let (days, seconds) = if seconds < 0 {
                let days = seconds.div_euclid(86400);
                let seconds = seconds.rem_euclid(86400);
                (days, seconds)
            } else {
                (0, seconds)
            };
            let delta = PyDelta::new(py, days, seconds, 0, false)?;
            PyTzInfo::fixed_offset(py, delta)
        }
    }
}

#[must_use]
pub(crate) fn normalize_line_ending(s: &'_ str) -> Cow<'_, str> {
    if !s.contains('\r') {
        return Cow::Borrowed(s);
    }

    let mut s = s.to_string();
    let bytes = unsafe { s.as_bytes_mut() };
    let mut i = 0;
    let mut write = 0;

    while i < bytes.len() {
        if bytes[i] == b'\r' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                bytes[write] = b'\n';
                write += 1;
                i += 2;
            } else {
                bytes[write] = b'\r';
                write += 1;
                i += 1;
            }
        } else {
            bytes[write] = bytes[i];
            write += 1;
            i += 1;
        }
    }

    s.truncate(write);
    Cow::Owned(s)
}
