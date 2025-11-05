use std::borrow::Cow;

use pyo3::{
    IntoPyObjectExt,
    exceptions::{PyRecursionError, PyValueError},
    intern,
    prelude::*,
    types::{self as t, PyDateAccess, PyDeltaAccess, PyTimeAccess, PyTzInfoAccess},
};
use smallvec::SmallVec;
use toml::{Value, value::Offset};

#[derive(Copy, Clone, Debug)]
struct Limit(usize);

impl Limit {
    #[inline]
    fn _limit(&self, value: usize) -> bool {
        value < self.0
    }
}

const RECURSION_LIMIT: Limit = Limit(999);

#[derive(Clone, Debug)]
struct RecursionGuard {
    current: usize,
    limit: Limit,
}

impl Default for RecursionGuard {
    fn default() -> Self {
        Self {
            current: 0,
            limit: RECURSION_LIMIT,
        }
    }
}

impl RecursionGuard {
    #[inline(always)]
    fn enter(&mut self) -> PyResult<()> {
        if !self.limit._limit(self.current) {
            return Err(PyRecursionError::new_err(
                "max recursion depth met".to_string(),
            ));
        }
        self.current += 1;
        Ok(())
    }

    #[inline(always)]
    fn exit(&mut self) {
        self.current -= 1;
    }
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
    match value {
        Value::String(str) => str.into_bound_py_any(py),
        Value::Integer(int) => int.into_bound_py_any(py),
        Value::Float(float) => {
            if let Some(f) = parse_float {
                let mut ryu_buf = ryu::Buffer::new();
                let py_call = f.call1((ryu_buf.format(float),))?;
                if py_call.cast::<t::PyDict>().is_ok() || py_call.cast::<t::PyList>().is_ok() {
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
        Value::Array(array) => {
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
        Value::Table(table) => {
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
    offset: &Offset,
) -> PyResult<Bound<'py, t::PyTzInfo>> {
    match offset {
        Offset::Z => t::PyTzInfo::utc(py).map(|utc| utc.to_owned()),
        Offset::Custom { minutes } => {
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

pub(crate) fn python_to_toml<'py>(py: Python<'py>, obj: &Bound<'py, PyAny>) -> PyResult<Value> {
    _python_to_toml(py, obj, &mut RecursionGuard::default())
}

fn _python_to_toml<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    recursion: &mut RecursionGuard,
) -> PyResult<Value> {
    if let Ok(str) = obj.cast::<t::PyString>() {
        return Ok(Value::String(str.to_str()?.to_owned()));
    } else if let Ok(bool) = obj.cast::<t::PyBool>() {
        return Ok(Value::Boolean(bool.is_true()));
    } else if let Ok(int) = obj.cast::<t::PyInt>() {
        return Ok(Value::Integer(int.extract()?));
    } else if let Ok(float) = obj.cast::<t::PyFloat>() {
        return Ok(Value::Float(float.value()));
    }

    if let Ok(dt) = obj.cast::<t::PyDateTime>() {
        let date = crate::toml_dt!(Date, dt.get_year(), dt.get_month(), dt.get_day());
        let time = crate::toml_dt!(
            Time,
            dt.get_hour(),
            dt.get_minute(),
            dt.get_second(),
            dt.get_microsecond() * 1000
        );

        let offset = if let Some(tzinfo) = dt.get_tzinfo() {
            let utc_offset = tzinfo.call_method1(intern!(py, "utcoffset"), (dt,))?;
            if utc_offset.is_none() {
                None
            } else {
                let delta = utc_offset.cast::<t::PyDelta>()?;
                let seconds = delta.get_days() * 86400 + delta.get_seconds();
                Some(Offset::Custom {
                    minutes: (seconds / 60) as i16,
                })
            }
        } else {
            None
        };

        return Ok(crate::toml_dt!(Datetime, Some(date), Some(time), offset));
    } else if let Ok(date_obj) = obj.cast::<t::PyDate>() {
        let date = crate::toml_dt!(
            Date,
            date_obj.get_year(),
            date_obj.get_month(),
            date_obj.get_day()
        );
        return Ok(crate::toml_dt!(Datetime, Some(date), None, None));
    } else if let Ok(time_obj) = obj.cast::<t::PyTime>() {
        let time = crate::toml_dt!(
            Time,
            time_obj.get_hour(),
            time_obj.get_minute(),
            time_obj.get_second(),
            time_obj.get_microsecond() * 1000
        );
        return Ok(crate::toml_dt!(Datetime, None, Some(time), None));
    }

    if let Ok(dict) = obj.cast::<t::PyDict>() {
        recursion.enter()?;
        let len = dict.len();

        if len == 0 {
            return Ok(Value::Table(toml::map::Map::new()));
        }

        let mut items: SmallVec<[(String, Value); 8]> = SmallVec::with_capacity(len);

        for (k, v) in dict.iter() {
            let key = k
                .cast::<t::PyString>()
                .map_err(|_| {
                    crate::TOMLEncodeError::new_err(format!(
                        "TOML table keys must be strings, got {}",
                        crate::get_type!(k)
                    ))
                })?
                .to_str()?
                .to_owned();

            let value = _python_to_toml(py, &v, recursion)?;
            items.push((key, value));
        }
        let mut table = toml::map::Map::with_capacity(len);

        for (k, v) in items {
            table.insert(k, v);
        }
        recursion.exit();
        return Ok(Value::Table(table));
    }

    if let Ok(list) = obj.cast::<t::PyList>() {
        recursion.enter()?;
        let len = list.len();

        if len == 0 {
            return Ok(Value::Array(Vec::new()));
        }

        let mut items: SmallVec<[Value; 8]> = SmallVec::with_capacity(len);

        for item in list.iter() {
            items.push(_python_to_toml(py, &item, recursion)?);
        }
        recursion.exit();
        return Ok(Value::Array(items.into_vec()));
    }

    Err(crate::TOMLEncodeError::new_err(format!(
        "Cannot serialize {} to TOML",
        crate::get_type!(obj)
    )))
}
