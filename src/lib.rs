use pyo3::types::PyBytes;
use pyo3::{
    IntoPyObjectExt,
    exceptions::{PyTypeError, PyValueError},
    prelude::*,
    types::{PyDate, PyDateTime, PyDelta, PyDict, PyList, PyTime, PyTzInfo},
};
use toml_datetime::Offset;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pyo3::create_exception!(
    toml_rs,
    TOMLDecodeError,
    PyValueError,
    "An error raised if a document is not valid TOML."
);

fn convert_toml(
    py: Python,
    value: toml::Value,
    parse_float: Option<&Py<PyAny>>,
) -> PyResult<Py<PyAny>> {
    match value {
        toml::Value::String(str) => Ok(str.into_py_any(py)?),
        toml::Value::Integer(int) => Ok(int.into_py_any(py)?),
        toml::Value::Float(float) => {
            if let Some(_parse_float) = parse_float {
                let result = _parse_float.call1(py, (float.to_string(),))?;
                let bound = result.bind(py);

                if bound.is_instance_of::<PyDict>() || bound.is_instance_of::<PyList>() {
                    return Err(PyValueError::new_err(
                        "parse_float must not return dicts or lists",
                    ));
                }

                Ok(result)
            } else {
                Ok(float.into_py_any(py)?)
            }
        }
        toml::Value::Boolean(bool) => Ok(bool.into_py_any(py)?),
        toml::Value::Array(array) => {
            let mut values = Vec::with_capacity(array.len());
            for item in array {
                values.push(convert_toml(py, item, parse_float)?);
            }
            Ok(PyList::new(py, values)?.into())
        }
        toml::Value::Table(table) => {
            let dict = PyDict::new(py);
            for (k, v) in table {
                let value = convert_toml(py, v, parse_float)?;
                dict.set_item(k, value)?;
            }
            Ok(dict.into())
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
                Ok(py_datetime.into())
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
                Ok(py_datetime.into())
            }
            (Some(date), None, None) => {
                let py_date = PyDate::new(py, date.year as i32, date.month, date.day)?;
                Ok(py_date.into())
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
                Ok(py_time.into())
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
        Offset::Z => Ok(PyTzInfo::utc(py)?.to_owned()),
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

#[pyfunction]
fn _loads(py: Python, s: &str, parse_float: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
    let value = py
        .detach(|| toml::from_str(s))
        .map_err(|err| TOMLDecodeError::new_err(format!("{}", err)))?;
    convert_toml(py, value, parse_float.as_ref())
}

#[pyfunction]
fn _load(py: Python, fp: Py<PyAny>, parse_float: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
    let bound = fp.bind(py);
    let read = bound.getattr("read")?;
    let content_obj = read.call0()?;

    if let Ok(bytes) = content_obj.cast::<PyBytes>() {
        match toml::from_slice(bytes.as_bytes()) {
            Ok(value) => convert_toml(py, value, parse_float.as_ref()),
            Err(e) => Err(TOMLDecodeError::new_err(format!("{}", e))),
        }
    } else if let Ok(s) = content_obj.extract::<&str>() {
        _loads(py, s, parse_float)
    } else {
        Err(PyErr::new::<PyTypeError, _>(
            "Expected str or bytes-like object",
        ))
    }
}

#[pymodule]
fn _toml_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(_load, m)?)?;
    m.add_function(wrap_pyfunction!(_loads, m)?)?;
    m.add("_version", env!("CARGO_PKG_VERSION"))?;
    m.add("TOMLDecodeError", m.py().get_type::<TOMLDecodeError>())?;
    Ok(())
}
