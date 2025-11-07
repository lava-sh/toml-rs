use crate::recursion_guard::RecursionGuard;

use pyo3::{
    intern,
    prelude::*,
    types::{self as t, PyDateAccess, PyDeltaAccess, PyTimeAccess, PyTzInfoAccess},
};
use rustc_hash::FxHashSet;
use smallvec::SmallVec;
use toml_edit::{Array, Formatted, InlineTable, Item, Table, Value};

pub(crate) fn validate_inline_paths(
    doc: &Item,
    inline_tables: &FxHashSet<String>,
) -> Result<(), PyErr> {
    for path in inline_tables {
        let mut current = doc;

        for key in path.split(".") {
            if let Some(item) = current.get(key) {
                current = item;
            } else {
                return Err(crate::TOMLEncodeError::new_err(format!(
                    "Path '{}' specified in inline_tables does not exist in the toml",
                    path
                )));
            }
        }

        if !current.is_table() && !current.is_inline_table() {
            return Err(crate::TOMLEncodeError::new_err(format!(
                "Path '{}' does not point to a table",
                path
            )));
        }
    }

    Ok(())
}

pub(crate) fn python_to_toml<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    inline_tables: Option<&FxHashSet<String>>,
) -> PyResult<Item> {
    _python_to_toml(
        py,
        obj,
        &mut RecursionGuard::default(),
        inline_tables,
        &mut SmallVec::<[String; 8]>::new(),
    )
}

fn _python_to_toml<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    recursion: &mut RecursionGuard,
    inline_tables: Option<&FxHashSet<String>>,
    _path: &mut SmallVec<[String; 8]>,
) -> PyResult<Item> {
    if let Ok(str) = obj.cast::<t::PyString>() {
        return Ok(Item::Value(Value::String(Formatted::new(
            str.to_str()?.to_owned(),
        ))));
    } else if let Ok(bool) = obj.cast::<t::PyBool>() {
        return Ok(Item::Value(Value::Boolean(Formatted::new(bool.is_true()))));
    } else if let Ok(int) = obj.cast::<t::PyInt>() {
        return Ok(Item::Value(Value::Integer(Formatted::new(int.extract()?))));
    } else if let Ok(float) = obj.cast::<t::PyFloat>() {
        return Ok(Item::Value(Value::Float(Formatted::new(float.value()))));
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

        return Ok(Item::Value(Value::Datetime(Formatted::new(
            crate::toml_dt!(Datetime, Some(date), Some(time), offset),
        ))));
    } else if let Ok(date_obj) = obj.cast::<t::PyDate>() {
        let date = crate::toml_dt!(
            Date,
            date_obj.get_year(),
            date_obj.get_month(),
            date_obj.get_day()
        );
        return Ok(Item::Value(Value::Datetime(Formatted::new(
            crate::toml_dt!(Datetime, Some(date), None, None),
        ))));
    } else if let Ok(time_obj) = obj.cast::<t::PyTime>() {
        let time = crate::toml_dt!(
            Time,
            time_obj.get_hour(),
            time_obj.get_minute(),
            time_obj.get_second(),
            time_obj.get_microsecond() * 1000
        );
        return Ok(Item::Value(Value::Datetime(Formatted::new(
            crate::toml_dt!(Datetime, None, Some(time), None),
        ))));
    }

    if let Ok(dict) = obj.cast::<t::PyDict>() {
        recursion.enter()?;

        if dict.is_empty() {
            recursion.exit();
            return Ok(Item::Table(Table::new()));
        }

        let inline = inline_tables
            .map(|set| set.contains(&_path.join(".")))
            .unwrap_or(false);

        return if inline {
            let mut inline_table = InlineTable::new();
            for (k, v) in dict.iter() {
                let key = k
                    .cast::<t::PyString>()
                    .map_err(|_| {
                        crate::TOMLEncodeError::new_err(format!(
                            "TOML table keys must be strings, got {}",
                            crate::get_type!(k)
                        ))
                    })?
                    .to_str()?;

                _path.push(key.to_owned());
                let item = _python_to_toml(py, &v, recursion, inline_tables, _path)?;
                _path.pop();

                if let Item::Value(val) = item {
                    inline_table.insert(key, val);
                } else {
                    recursion.exit();
                    return Err(crate::TOMLEncodeError::new_err(
                        "Inline tables can only contain values, not nested tables",
                    ));
                }
            }
            recursion.exit();
            Ok(Item::Value(Value::InlineTable(inline_table)))
        } else {
            let mut table = Table::new();
            for (k, v) in dict.iter() {
                let key = k
                    .cast::<t::PyString>()
                    .map_err(|_| {
                        crate::TOMLEncodeError::new_err(format!(
                            "TOML table keys must be strings, got {}",
                            crate::get_type!(k)
                        ))
                    })?
                    .to_str()?;

                _path.push(key.to_owned());
                let item = _python_to_toml(py, &v, recursion, inline_tables, _path)?;
                _path.pop();

                table.insert(key, item);
            }
            recursion.exit();
            Ok(Item::Table(table))
        };
    }

    if let Ok(list) = obj.cast::<t::PyList>() {
        recursion.enter()?;

        if list.is_empty() {
            recursion.exit();
            return Ok(Item::Value(Value::Array(Array::new())));
        }

        let mut array = Array::new();
        for item in list.iter() {
            let _item = _python_to_toml(py, &item, recursion, inline_tables, _path)?;
            match _item {
                Item::Value(value) => {
                    array.push(value);
                }
                Item::Table(table) => {
                    let inline_table = table.into_inline_table();
                    array.push(Value::InlineTable(inline_table));
                }
                _ => {
                    recursion.exit();
                    return Err(crate::TOMLEncodeError::new_err(
                        "Arrays can only contain values or inline tables",
                    ));
                }
            }
        }
        recursion.exit();
        return Ok(Item::Value(Value::Array(array)));
    }

    Err(crate::TOMLEncodeError::new_err(format!(
        "Cannot serialize {} to TOML",
        crate::get_type!(obj)
    )))
}
