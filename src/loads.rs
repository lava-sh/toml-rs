use crate::recursion_guard::RecursionGuard;

use std::borrow::Cow;

use pyo3::{IntoPyObjectExt, exceptions::PyValueError, prelude::*, types as t};

pub(crate) fn toml_to_python<'py>(
    py: Python<'py>,
    value: toml::Value,
    parse_float: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    _toml_to_python(py, value, parse_float, &mut RecursionGuard::default())
}

fn _toml_to_python<'py>(
    py: Python<'py>,
    value: toml::Value,
    parse_float: Option<&Bound<'py, PyAny>>,
    recursion: &mut RecursionGuard,
) -> PyResult<Bound<'py, PyAny>> {
    match value {
        toml::Value::String(str) => str.into_bound_py_any(py),
        toml::Value::Integer(int) => int.into_bound_py_any(py),
        toml::Value::Float(float) => {
            if let Some(f) = parse_float {
                let mut ryu_buf = ryu::Buffer::new();
                let py_call = f.call1((ryu_buf.format(float),))?;
                if py_call.is_instance_of::<t::PyDict>() || py_call.is_instance_of::<t::PyList>() {
                    return Err(PyValueError::new_err(
                        "parse_float must not return dicts or lists",
                    ));
                }
                Ok(py_call)
            } else {
                float.into_bound_py_any(py)
            }
        }
        toml::Value::Boolean(bool) => bool.into_bound_py_any(py),
        toml::Value::Datetime(datetime) => match (datetime.date, datetime.time, datetime.offset) {
            (Some(date), Some(time), Some(offset)) => {
                let tzinfo = Some(&create_timezone_from_offset(py, &offset)?);
                Ok(crate::create_py_datetime!(py, date, time, tzinfo)?.into_any())
            }
            (Some(date), Some(time), None) => {
                Ok(crate::create_py_datetime!(py, date, time, None)?.into_any())
            }
            (Some(date), None, None) => {
                let py_date = t::PyDate::new(py, date.year as i32, date.month, date.day)?;
                Ok(py_date.into_any())
            }
            (None, Some(time), None) => {
                let py_time = t::PyTime::new(
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
        toml::Value::Array(array) => {
            if array.is_empty() {
                return Ok(t::PyList::empty(py).into_any());
            }

            recursion.enter()?;
            let py_list = t::PyList::empty(py);
            for item in array {
                py_list.append(_toml_to_python(py, item, parse_float, recursion)?)?;
            }
            recursion.exit();
            Ok(py_list.into_any())
        }
        toml::Value::Table(table) => {
            if table.is_empty() {
                return Ok(t::PyDict::new(py).into_any());
            }

            recursion.enter()?;
            let py_dict = t::PyDict::new(py);
            for (k, v) in table {
                let value = _toml_to_python(py, v, parse_float, recursion)?;
                py_dict.set_item(k, value)?;
            }
            recursion.exit();
            Ok(py_dict.into_any())
        }
    }
}

fn create_timezone_from_offset<'py>(
    py: Python<'py>,
    offset: &toml::value::Offset,
) -> PyResult<Bound<'py, t::PyTzInfo>> {
    match offset {
        toml::value::Offset::Z => t::PyTzInfo::utc(py).map(|utc| utc.to_owned()),
        toml::value::Offset::Custom { minutes } => {
            let seconds = *minutes as i32 * 60;
            let (days, seconds) = if seconds < 0 {
                let days = seconds.div_euclid(86400);
                let seconds = seconds.rem_euclid(86400);
                (days, seconds)
            } else {
                (0, seconds)
            };
            let py_delta = t::PyDelta::new(py, days, seconds, 0, false)?;
            t::PyTzInfo::fixed_offset(py, py_delta)
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
