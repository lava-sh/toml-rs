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
use toml_edit::{Array, InlineTable, Item, Offset, Table, Value};

use crate::{TOMLEncodeError, get_type, recursion_guard::RecursionGuard, to_toml, toml_dt};

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
    to_toml(
        py,
        obj,
        &mut RecursionGuard::default(),
        inline_tables,
        &mut SmallVec::<String, 16>::with_capacity(inline_tables.map_or(0, FxHashSet::len)),
    )
}

fn to_toml<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
    recursion: &mut RecursionGuard,
    inline_tables: Option<&FxHashSet<String>>,
    toml_path: &mut SmallVec<String, 16>,
) -> PyResult<Item> {
    if let Ok(s) = obj.cast::<PyString>() {
        return to_toml!(String, s.to_str()?.to_owned());
    }
    if let Ok(b) = obj.cast::<PyBool>() {
        return to_toml!(Boolean, b.is_true());
    }
    if let Ok(int) = obj.cast::<PyInt>() {
        return to_toml!(Integer, int.extract()?);
    }
    if let Ok(float) = obj.cast::<PyFloat>() {
        return to_toml!(Float, float.value());
    }

    if let Ok(py_datetime) = obj.cast::<PyDateTime>() {
        let date = toml_dt!(Date, py_datetime);
        let time = toml_dt!(Time, py_datetime);

        let offset = py_datetime.get_tzinfo().and_then(|tzinfo| {
            let utc_offset = tzinfo
                .call_method1(intern!(py, "utcoffset"), (py_datetime,))
                .ok()?;
            if utc_offset.is_none() {
                return None;
            }
            let delta = utc_offset.cast::<PyDelta>().ok()?;
            let seconds = delta.get_days() * 86400 + delta.get_seconds();
            Some(Offset::Custom {
                minutes: i16::try_from(seconds / 60).ok()?,
            })
        });

        let datetime = toml_dt!(Datetime, Some(date), Some(time), offset);
        return to_toml!(Datetime, datetime);
    } else if let Ok(py_date) = obj.cast::<PyDate>() {
        let date = toml_dt!(Date, py_date);
        let datetime = toml_dt!(Datetime, Some(date), None, None);
        return to_toml!(Datetime, datetime);
    } else if let Ok(py_time) = obj.cast::<PyTime>() {
        let time = toml_dt!(Time, py_time);
        let datetime = toml_dt!(Datetime, None, Some(time), None);
        return to_toml!(Datetime, datetime);
    }

    if let Ok(dict) = obj.cast::<PyDict>() {
        recursion.enter()?;

        if dict.is_empty() {
            recursion.exit();
            return to_toml!(TomlTable, Table::new());
        }

        let inline = inline_tables.is_some_and(|set| set.contains(&toml_path.join(".")));

        return if inline {
            let mut inline_table = InlineTable::new();
            for (k, v) in dict.iter() {
                let key = k
                    .cast::<PyString>()
                    .map_err(|_| {
                        TOMLEncodeError::new_err(format!(
                            "TOML table keys must be strings, got {py_type}",
                            py_type = get_type!(k)
                        ))
                    })?
                    .to_str()?;

                toml_path.push(key.to_owned());
                let item = to_toml(py, &v, recursion, inline_tables, toml_path)?;
                toml_path.pop();

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
            to_toml!(TomlInlineTable, inline_table)
        } else {
            let mut table = Table::new();
            for (k, v) in dict.iter() {
                let key = k
                    .cast::<PyString>()
                    .map_err(|_| {
                        TOMLEncodeError::new_err(format!(
                            "TOML table keys must be strings, got {py_type}",
                            py_type = get_type!(obj)
                        ))
                    })?
                    .to_str()?;

                toml_path.push(key.to_owned());
                let item = to_toml(py, &v, recursion, inline_tables, toml_path)?;
                toml_path.pop();

                table.insert(key, item);
            }
            recursion.exit();
            to_toml!(TomlTable, table)
        };
    }

    if let Ok(list) = obj.cast::<PyList>() {
        recursion.enter()?;

        if list.is_empty() {
            recursion.exit();
            return to_toml!(TomlArray, Array::new());
        }

        let mut array = Array::new();
        for item in list.iter() {
            let items = to_toml(py, &item, recursion, inline_tables, toml_path)?;
            match items {
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
        return to_toml!(TomlArray, array);
    }

    Err(TOMLEncodeError::new_err(format!(
        "Cannot serialize {py_type} to TOML",
        py_type = get_type!(obj)
    )))
}
