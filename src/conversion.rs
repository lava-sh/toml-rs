use std::borrow::Cow;

use pyo3::{
    IntoPyObjectExt,
    exceptions::{PyRecursionError, PyValueError},
    intern,
    prelude::*,
    types::{
        PyBool, PyDate, PyDateAccess, PyDateTime, PyDelta, PyDeltaAccess, PyDict, PyFloat, PyInt,
        PyList, PyString, PyTime, PyTimeAccess, PyTzInfo, PyTzInfoAccess,
    },
};
use toml::Value;
use toml_datetime::Offset;

const MAX_RECURSION_DEPTH: usize = 999;

#[derive(Clone, Debug, Default)]
struct RecursionGuard {
    current: usize,
}

impl RecursionGuard {
    #[inline(always)]
    fn enter(&mut self) -> PyResult<()> {
        self.current += 1;
        if MAX_RECURSION_DEPTH <= self.current {
            return Err(PyRecursionError::new_err(
                "max recursion depth met".to_string(),
            ));
        }
        Ok(())
    }

    #[inline(always)]
    fn exit(&mut self) {
        self.current -= 1;
    }
}

pub(crate) fn python_to_toml<'py>(py: Python<'py>, obj: &Bound<'py, PyAny>) -> PyResult<Value> {
    _python_to_toml(py, obj, &mut RecursionGuard::default())
}

pub(crate) fn toml_to_python<'py>(
    py: Python<'py>,
    value: Value,
    parse_float: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    _toml_to_python(py, value, parse_float, &mut RecursionGuard::default())
}

fn _toml_to_python<'py>(
    py: Python<'py>,
    value: Value,
    parse_float: Option<&Bound<'py, PyAny>>,
    recursion: &mut RecursionGuard,
) -> PyResult<Bound<'py, PyAny>> {
    recursion.enter()?;

    let toml = match value {
        Value::String(str) => str.into_bound_py_any(py),
        Value::Integer(int) => int.into_bound_py_any(py),
        Value::Float(float) => {
            if let Some(f) = parse_float {
                let mut buf = ryu::Buffer::new();
                let py_call = f.call1((buf.format(float),))?;
                if py_call.cast::<PyDict>().is_ok() || py_call.cast::<PyList>().is_ok() {
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
        Value::Array(array) => {
            let py_list = PyList::empty(py);
            for item in array {
                py_list.append(_toml_to_python(py, item, parse_float, recursion)?)?;
            }
            Ok(py_list.into_any())
        }
        Value::Table(table) => {
            let py_dict = PyDict::new(py);
            for (k, v) in table {
                let value = _toml_to_python(py, v, parse_float, recursion)?;
                py_dict.set_item(k, value)?;
            }
            Ok(py_dict.into_any())
        }
        Value::Datetime(datetime) => match (datetime.date, datetime.time, datetime.offset) {
            (Some(date), Some(time), Some(offset)) => {
                let tzinfo = Some(&create_timezone_from_offset(py, &offset)?);
                Ok(crate::create_py_datetime!(py, date, time, tzinfo)?.into_any())
            }
            (Some(date), Some(time), None) => {
                Ok(crate::create_py_datetime!(py, date, time, None)?.into_any())
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
    recursion.exit();
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

#[inline]
fn _python_to_toml<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    recursion: &mut RecursionGuard,
) -> PyResult<Value> {
    recursion.enter()?;

    let value = if let Ok(str) = obj.cast::<PyString>() {
        Value::String(str.to_string())
    } else if let Ok(bool) = obj.cast::<PyBool>() {
        Value::Boolean(bool.is_true())
    } else if let Ok(int) = obj.cast::<PyInt>() {
        Value::Integer(int.extract()?)
    } else if let Ok(float) = obj.cast::<PyFloat>() {
        Value::Float(float.value())
    } else if let Ok(dict) = obj.cast::<PyDict>() {
        let mut table = toml::map::Map::with_capacity(dict.len());
        for (k, v) in dict.iter() {
            let key = k
                .cast::<PyString>()
                .map_err(|_| {
                    crate::TOMLEncodeError::new_err((
                        format!(
                            "TOML table keys must be strings, got {}",
                            crate::get_type!(k)
                        ),
                        None::<Py<PyAny>>,
                    ))
                })?
                .to_string();
            table.insert(key, _python_to_toml(py, &v, recursion)?);
        }
        Value::Table(table)
    } else if let Ok(list) = obj.cast::<PyList>() {
        let mut vec = Vec::with_capacity(list.len());
        for item in list.iter() {
            vec.push(_python_to_toml(py, &item, recursion)?);
        }
        Value::Array(vec)
    } else if let Ok(dt) = obj.cast::<PyDateTime>() {
        Value::Datetime(toml_datetime::Datetime {
            date: Some(toml_datetime::Date {
                year: dt.get_year() as u16,
                month: dt.get_month(),
                day: dt.get_day(),
            }),
            time: Some(toml_datetime::Time {
                hour: dt.get_hour(),
                minute: dt.get_minute(),
                second: dt.get_second(),
                nanosecond: dt.get_microsecond() * 1000,
            }),
            offset: if let Some(tzinfo) = dt.get_tzinfo() {
                let utc_offset = tzinfo.call_method1(intern!(py, "utcoffset"), (dt,))?;
                if utc_offset.is_none() {
                    None
                } else {
                    let delta = utc_offset.cast::<PyDelta>()?;
                    let total_seconds = delta.get_days() * 86400 + delta.get_seconds();
                    Some(Offset::Custom {
                        minutes: (total_seconds / 60) as i16,
                    })
                }
            } else {
                None
            },
        })
    } else if let Ok(date) = obj.cast::<PyDate>() {
        Value::Datetime(toml_datetime::Datetime {
            date: Some(toml_datetime::Date {
                year: date.get_year() as u16,
                month: date.get_month(),
                day: date.get_day(),
            }),
            time: None,
            offset: None,
        })
    } else if let Ok(time) = obj.cast::<PyTime>() {
        Value::Datetime(toml_datetime::Datetime {
            date: None,
            time: Some(toml_datetime::Time {
                hour: time.get_hour(),
                minute: time.get_minute(),
                second: time.get_second(),
                nanosecond: time.get_microsecond() * 1000,
            }),
            offset: None,
        })
    } else {
        return Err(crate::TOMLEncodeError::new_err((
            format!("Cannot serialize {} to TOML", crate::get_type!(obj)),
            None::<Py<PyAny>>,
        )));
    };
    recursion.exit();
    Ok(value)
}
