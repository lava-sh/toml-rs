use std::{borrow::Cow, str::from_utf8_unchecked};

use pyo3::{
    IntoPyObjectExt,
    exceptions::PyValueError,
    prelude::*,
    types::{PyDate, PyDelta, PyDict, PyList, PyTime, PyTzInfo},
};
use toml::{Value, value::Offset};

use crate::{create_py_datetime, recursion_guard::RecursionGuard};

pub(crate) fn toml_to_python<'py>(
    py: Python<'py>,
    value: Value,
    parse_float: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    _toml_to_python(py, value, parse_float, &mut RecursionGuard::default())
}

#[inline]
fn _toml_to_python<'py>(
    py: Python<'py>,
    value: Value,
    parse_float: Option<&Bound<'py, PyAny>>,
    recursion: &mut RecursionGuard,
) -> PyResult<Bound<'py, PyAny>> {
    match value {
        Value::String(str) => str.into_bound_py_any(py),
        Value::Integer(int) => int.into_bound_py_any(py),
        Value::Float(float) => {
            if let Some(f) = parse_float {
                let mut buffer = [0u8; lexical_core::BUFFER_SIZE];
                let write_bytes = lexical_core::write(float, &mut buffer);
                let py_call = f.call1((
                    // SAFETY: `lexical_core::write()` guarantees that it only writes valid
                    // ASCII characters: 0-9, '.', '-' and 'e' for exponential notation.
                    // All these characters are valid UTF-8.
                    unsafe { from_utf8_unchecked(write_bytes) },
                ))?;
                if py_call.is_exact_instance_of::<PyDict>()
                    || py_call.is_exact_instance_of::<PyList>()
                {
                    return Err(PyValueError::new_err(
                        "parse_float must not return dicts or lists",
                    ));
                }
                Ok(py_call)
            } else {
                float.into_bound_py_any(py)
            }
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
                py_list.append(_toml_to_python(py, item, parse_float, recursion)?)?;
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
                let value = _toml_to_python(py, v, parse_float, recursion)?;
                py_dict.set_item(k, value)?;
            }
            recursion.exit();
            Ok(py_dict.into_any())
        }
    }
}

fn create_timezone_from_offset(
    py: Python,
    offset: Offset,
) -> PyResult<Bound<PyTzInfo>> {
    match offset {
        Offset::Z => PyTzInfo::utc(py).map(Borrowed::to_owned),
        Offset::Custom { minutes } => {
            let seconds = i32::from(minutes) * 60;
            let (days, seconds) = if seconds < 0 {
                let days = seconds.div_euclid(86400);
                let seconds = seconds.rem_euclid(86400);
                (days, seconds)
            } else {
                (0, seconds)
            };
            let py_delta = PyDelta::new(py, days, seconds, 0, false)?;
            PyTzInfo::fixed_offset(py, py_delta)
        }
    }
}

#[must_use]
pub(crate) fn normalize_line_ending(s: &'_ str) -> Cow<'_, str> {
    if memchr::memchr(b'\r', s.as_bytes()).is_none() {
        return Cow::Borrowed(s);
    }

    let mut buf = s.to_string().into_bytes();
    let mut gap_len = 0;
    let mut tail = buf.as_mut_slice();

    let finder = memchr::memmem::Finder::new(b"\r\n");

    loop {
        let idx = match finder.find(&tail[gap_len..]) {
            None => tail.len(),
            Some(idx) => idx + gap_len,
        };
        tail.copy_within(gap_len..idx, 0);
        tail = &mut tail[idx - gap_len..];

        if tail.len() == gap_len {
            break;
        }
        gap_len += 1;
    }
    // Account for removed `\r`.
    let new_len = buf.len() - gap_len;
    unsafe {
        // SAFETY: After `set_len`, `buf` is guaranteed to contain utf-8 again.
        buf.set_len(new_len);
        Cow::Owned(String::from_utf8_unchecked(buf))
    }
}
