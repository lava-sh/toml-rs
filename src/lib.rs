use pyo3::{
    IntoPyObjectExt,
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
    types::{PyBytes, PyDate, PyDateTime, PyDelta, PyDict, PyList, PyTime, PyTzInfo},
};
use toml_datetime::Offset;

#[cfg(not(any(
    all(target_os = "linux", target_arch = "aarch64"),
    all(target_os = "linux", target_arch = "arm"),
    all(target_os = "linux", target_arch = "s390x"),
    all(target_os = "linux", target_arch = "powerpc64")
)))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn convert_toml<'py>(
    py: Python<'py>,
    value: toml::Value,
    parse_float: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    match value {
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
                values.push(convert_toml(py, item, parse_float)?);
            }
            Ok(PyList::new(py, values)?.into_any())
        }
        toml::Value::Table(table) => {
            let dict = PyDict::new(py);
            for (k, v) in table {
                let value = convert_toml(py, v, parse_float)?;
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
    }
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

fn normalize_line_ending(s: &str) -> String {
    if !s.contains('\r') {
        return s.to_string();
    }

    let mut r = s.to_string();
    let bytes = unsafe { r.as_bytes_mut() };
    let mut write = 0;
    let mut i = 0;

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

    r.truncate(write);
    r
}

pyo3::import_exception!(toml_rs, TOMLDecodeError);

#[pyfunction]
fn _loads(py: Python, s: &str, parse_float: Option<Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
    let normalized = normalize_line_ending(s);
    let value = py
        .detach(|| toml::from_str(&normalized))
        .map_err(|err| TOMLDecodeError::new_err((err.to_string(), normalized.clone(), 0)))?;

    let result = convert_toml(py, value, parse_float.as_ref())?;
    Ok(result.unbind())
}

#[pyfunction]
fn _load(py: Python, fp: Py<PyAny>, parse_float: Option<Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
    let bound = fp.bind(py);
    let read = bound.getattr("read")?;
    let content_obj = read.call0()?;

    let s = if let Ok(bytes) = content_obj.cast::<PyBytes>() {
        match std::str::from_utf8(bytes.as_bytes()) {
            Ok(valid_str) => valid_str.to_string(),
            Err(_) => String::from_utf8_lossy(bytes.as_bytes()).into_owned(),
        }
    } else if content_obj.extract::<&str>().is_ok() {
        return Err(PyErr::new::<PyTypeError, _>(
            "File must be opened in binary mode, e.g. use `open('foo.toml', 'rb')`"
        ));
    } else {
        return Err(PyErr::new::<PyTypeError, _>(
            "Expected bytes-like object from .read()"
        ));
    };

    _loads(py, &s, parse_float)
}

#[pymodule]
fn _toml_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(_load, m)?)?;
    m.add_function(wrap_pyfunction!(_loads, m)?)?;
    m.add("_version", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
