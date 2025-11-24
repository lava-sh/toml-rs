use pyo3::{
    intern,
    prelude::*,
    types::{
        PyBool, PyDate, PyDateAccess, PyDateTime, PyDelta, PyDeltaAccess, PyDict, PyFloat, PyInt,
        PyList, PyString, PyTime, PyTimeAccess, PyTzInfoAccess,
    },
};
use rustc_hash::FxHashSet;
use smallvec::SmallVec;
use toml_edit::{Array, Formatted, InlineTable, Item, Offset, Table, Value};

use crate::{TOMLEncodeError, get_type, recursion_guard::RecursionGuard};

pub(crate) fn validate_inline_paths(
    doc: &Item,
    inline_tables: &FxHashSet<String>,
) -> Result<(), PyErr> {
    for path in inline_tables {
        let mut current = doc;

        for key in path.split('.') {
            let Some(item) = current.get(key) else {
                return Err(TOMLEncodeError::new_err(format!(
                    "Path '{path}' specified in inline_tables does not exist in the toml"
                )));
            };
            current = item;
        }

        if !current.is_table() && !current.is_inline_table() {
            return Err(TOMLEncodeError::new_err(format!(
                "Path '{path}' does not point to a table",
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
        &mut SmallVec::<String, 32>::with_capacity(inline_tables.map_or(0, FxHashSet::len)),
    )
}

fn _python_to_toml<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    recursion: &mut RecursionGuard,
    inline_tables: Option<&FxHashSet<String>>,
    _path: &mut SmallVec<String, 32>,
) -> PyResult<Item> {
    if let Ok(str) = obj.cast::<PyString>() {
        return Ok(Item::Value(Value::String(Formatted::new(
            str.to_str()?.to_owned(),
        ))));
    }
    if let Ok(b) = obj.cast::<PyBool>() {
        return Ok(Item::Value(Value::Boolean(Formatted::new(b.is_true()))));
    }
    if let Ok(int) = obj.cast::<PyInt>() {
        return Ok(Item::Value(Value::Integer(Formatted::new(int.extract()?))));
    }
    if let Ok(float) = obj.cast::<PyFloat>() {
        return Ok(Item::Value(Value::Float(Formatted::new(float.value()))));
    }

    if let Ok(dt) = obj.cast::<PyDateTime>() {
        let date = crate::toml_dt!(Date, dt.get_year(), dt.get_month(), dt.get_day());
        let time = crate::toml_dt!(
            Time,
            dt.get_hour(),
            dt.get_minute(),
            dt.get_second(),
            dt.get_microsecond() * 1000
        );

        let offset = dt.get_tzinfo().and_then(|tzinfo| {
            let utc_offset = tzinfo.call_method1(intern!(py, "utcoffset"), (dt,)).ok()?;
            if utc_offset.is_none() {
                return None;
            }
            let delta = utc_offset.cast::<PyDelta>().ok()?;
            let seconds = delta.get_days() * 86400 + delta.get_seconds();
            Some(Offset::Custom {
                minutes: i16::try_from(seconds / 60).ok()?,
            })
        });

        return Ok(Item::Value(Value::Datetime(Formatted::new(
            crate::toml_dt!(Datetime, Some(date), Some(time), offset),
        ))));
    } else if let Ok(date_obj) = obj.cast::<PyDate>() {
        let date = crate::toml_dt!(
            Date,
            date_obj.get_year(),
            date_obj.get_month(),
            date_obj.get_day()
        );
        return Ok(Item::Value(Value::Datetime(Formatted::new(
            crate::toml_dt!(Datetime, Some(date), None, None),
        ))));
    } else if let Ok(time_obj) = obj.cast::<PyTime>() {
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

    if let Ok(dict) = obj.cast::<PyDict>() {
        recursion.enter()?;

        if dict.is_empty() {
            recursion.exit();
            return Ok(Item::Table(Table::new()));
        }

        let inline = inline_tables.is_some_and(|set| set.contains(&_path.join(".")));

        return if inline {
            let mut inline_table = InlineTable::new();
            for (k, v) in dict.iter() {
                let key = k
                    .cast::<PyString>()
                    .map_err(|_| {
                        TOMLEncodeError::new_err(format!(
                            "TOML table keys must be strings, got {}",
                            get_type!(k)
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
                    return Err(TOMLEncodeError::new_err(
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
                    .cast::<PyString>()
                    .map_err(|_| {
                        TOMLEncodeError::new_err(format!(
                            "TOML table keys must be strings, got {}",
                            get_type!(k)
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

    if let Ok(list) = obj.cast::<PyList>() {
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
                    return Err(TOMLEncodeError::new_err(
                        "Arrays can only contain values or inline tables",
                    ));
                }
            }
        }
        recursion.exit();
        return Ok(Item::Value(Value::Array(array)));
    }

    Err(TOMLEncodeError::new_err(format!(
        "Cannot serialize {} to TOML",
        get_type!(obj)
    )))
}
