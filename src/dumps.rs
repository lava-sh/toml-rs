use crate::recursion_guard::RecursionGuard;

use pyo3::{
    intern,
    prelude::*,
    types::{self as t, PyDateAccess, PyDeltaAccess, PyTimeAccess, PyTzInfoAccess},
};
use smallvec::SmallVec;

pub(crate) fn python_to_toml<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
) -> PyResult<toml::Value> {
    _python_to_toml(py, obj, &mut RecursionGuard::default())
}

fn _python_to_toml<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    recursion: &mut RecursionGuard,
) -> PyResult<toml::Value> {
    if let Ok(str) = obj.cast::<t::PyString>() {
        return Ok(toml::Value::String(str.to_str()?.to_owned()));
    } else if let Ok(bool) = obj.cast::<t::PyBool>() {
        return Ok(toml::Value::Boolean(bool.is_true()));
    } else if let Ok(int) = obj.cast::<t::PyInt>() {
        return Ok(toml::Value::Integer(int.extract()?));
    } else if let Ok(float) = obj.cast::<t::PyFloat>() {
        return Ok(toml::Value::Float(float.value()));
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
                Some(toml::value::Offset::Custom {
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
            return Ok(toml::Value::Table(toml::map::Map::new()));
        }

        let mut items: SmallVec<[(String, toml::Value); 8]> = SmallVec::with_capacity(len);

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
        return Ok(toml::Value::Table(table));
    }

    if let Ok(list) = obj.cast::<t::PyList>() {
        recursion.enter()?;
        let len = list.len();

        if len == 0 {
            return Ok(toml::Value::Array(Vec::new()));
        }

        let mut items: SmallVec<[toml::Value; 8]> = SmallVec::with_capacity(len);

        for item in list.iter() {
            items.push(_python_to_toml(py, &item, recursion)?);
        }
        recursion.exit();
        return Ok(toml::Value::Array(items.into_vec()));
    }

    Err(crate::TOMLEncodeError::new_err(format!(
        "Cannot serialize {} to TOML",
        crate::get_type!(obj)
    )))
}
