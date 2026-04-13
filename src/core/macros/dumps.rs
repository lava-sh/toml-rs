#[macro_export]
macro_rules! impl_dumps {
    (
        $validate_fn:ident,
        $python_to_toml_fn:ident,
        $to_toml_macro:ident,
        $toml_dt_macro:ident
    ) => {
        pub(crate) fn $validate_fn(
            doc: &Item,
            inline_tables: &rustc_hash::FxHashSet<String>,
        ) -> Result<(), pyo3::PyErr> {
            for path in inline_tables {
                let mut current = doc;

                for key in path.split('.') {
                    let Some(item) = current.get(key) else {
                        return Err($crate::toml_rs::TOMLEncodeError::new_err(format!(
                            "Path '{path}' specified in inline_tables does not exist in the toml"
                        )));
                    };
                    current = item;
                }

                if !current.is_table() && !current.is_inline_table() {
                    return Err($crate::toml_rs::TOMLEncodeError::new_err(format!(
                        "Path '{path}' does not point to a table",
                    )));
                }
            }

            Ok(())
        }

        pub(crate) fn $python_to_toml_fn<'py>(
            py: pyo3::Python<'py>,
            obj: &pyo3::Bound<'py, pyo3::PyAny>,
            inline_tables: Option<&rustc_hash::FxHashSet<String>>,
        ) -> pyo3::PyResult<Item> {
            to_toml_impl(
                py,
                obj,
                inline_tables,
                &mut smallvec::SmallVec::<String, 32>::with_capacity(
                    inline_tables.map_or(0, rustc_hash::FxHashSet::len),
                ),
            )
        }

        fn to_toml_impl<'py>(
            py: pyo3::Python<'py>,
            obj: &pyo3::Bound<'py, pyo3::PyAny>,
            inline_tables: Option<&rustc_hash::FxHashSet<String>>,
            toml_path: &mut smallvec::SmallVec<String, 32>,
        ) -> pyo3::PyResult<Item> {
            if let Ok(s) = obj.cast::<pyo3::types::PyString>() {
                return $to_toml_macro!(String, s.to_str()?.to_owned());
            }
            if let Ok(b) = obj.cast::<pyo3::types::PyBool>() {
                return $to_toml_macro!(Boolean, b.is_true());
            }
            if let Ok(int) = obj.cast::<pyo3::types::PyInt>() {
                return $to_toml_macro!(BigNum, int.str()?.to_str()?);
            }
            if let Ok(float) = obj.cast::<pyo3::types::PyFloat>() {
                return $to_toml_macro!(BigNum, float.str()?.to_str()?);
            }

            if let Ok(py_datetime) = obj.cast::<pyo3::types::PyDateTime>() {
                let date = $toml_dt_macro!(Date, py_datetime);
                let time = $toml_dt_macro!(Time, py_datetime);

                let offset = py_datetime.get_tzinfo().and_then(|tzinfo| {
                    let utc_offset = tzinfo
                        .call_method1(pyo3::intern!(py, "utcoffset"), (py_datetime,))
                        .ok()?;
                    if utc_offset.is_none() {
                        return None;
                    }
                    let delta = utc_offset.cast::<pyo3::types::PyDelta>().ok()?;
                    let seconds = delta.get_days() * 86400 + delta.get_seconds();
                    Some(Offset::Custom {
                        minutes: i16::try_from(seconds / 60).ok()?,
                    })
                });

                let datetime = $toml_dt_macro!(Datetime, Some(date), Some(time), offset);
                return $to_toml_macro!(Datetime, datetime);
            } else if let Ok(py_date) = obj.cast::<pyo3::types::PyDate>() {
                let date = $toml_dt_macro!(Date, py_date);
                let datetime = $toml_dt_macro!(Datetime, Some(date), None, None);
                return $to_toml_macro!(Datetime, datetime);
            } else if let Ok(py_time) = obj.cast::<pyo3::types::PyTime>() {
                let time = $toml_dt_macro!(Time, py_time);
                let datetime = $toml_dt_macro!(Datetime, None, Some(time), None);
                return $to_toml_macro!(Datetime, datetime);
            }

            if let Ok(dict) = obj.cast::<pyo3::types::PyDict>() {
                if dict.is_empty() {
                    return $to_toml_macro!(TomlTable, Table::new());
                }

                let inline = inline_tables.is_some_and(|set| set.contains(&toml_path.join(".")));

                return if inline {
                    let mut inline_table = InlineTable::new();
                    for (k, v) in dict.iter() {
                        let key = k
                            .cast::<pyo3::types::PyString>()
                            .map_err(|_| {
                                $crate::toml_rs::TOMLEncodeError::new_err(format!(
                                    "TOML table keys must be strings, got {py_type}",
                                    py_type = $crate::get_type!(k)
                                ))
                            })?
                            .to_str()?;

                        toml_path.push(key.to_owned());
                        let item = to_toml_impl(py, &v, inline_tables, toml_path)?;
                        toml_path.pop();

                        if let Item::Value(val) = item {
                            inline_table.insert(key, val);
                        } else {
                            return Err($crate::toml_rs::TOMLEncodeError::new_err(
                                "Inline tables can only contain values, not nested tables",
                            ));
                        }
                    }

                    $to_toml_macro!(TomlInlineTable, inline_table)
                } else {
                    let mut table = Table::new();
                    for (k, v) in dict.iter() {
                        let key = k
                            .cast::<pyo3::types::PyString>()
                            .map_err(|_| {
                                $crate::toml_rs::TOMLEncodeError::new_err(format!(
                                    "TOML table keys must be strings, got {py_type}",
                                    py_type = $crate::get_type!(k)
                                ))
                            })?
                            .to_str()?;

                        toml_path.push(key.to_owned());
                        let item = to_toml_impl(py, &v, inline_tables, toml_path)?;
                        toml_path.pop();

                        table.insert(key, item);
                    }
                    $to_toml_macro!(TomlTable, table)
                };
            }

            if let Ok(list) = obj.cast::<pyo3::types::PyList>() {
                if list.is_empty() {
                    return $to_toml_macro!(TomlArray, Array::new());
                }

                let mut array = Array::new();
                for item in list.iter() {
                    let items = to_toml_impl(py, &item, inline_tables, toml_path)?;
                    match items {
                        Item::Value(value) => {
                            array.push(value);
                        }
                        Item::Table(table) => {
                            let inline_table = table.into_inline_table();
                            array.push(Value::InlineTable(inline_table));
                        }
                        _ => {
                            return Err($crate::toml_rs::TOMLEncodeError::new_err(
                                "Arrays can only contain values or inline tables",
                            ));
                        }
                    }
                }

                return $to_toml_macro!(TomlArray, array);
            }

            if let Ok(py_tuple) = obj.cast::<pyo3::types::PyTuple>() {
                if py_tuple.is_empty() {
                    return $to_toml_macro!(TomlArray, Array::new());
                }

                let mut array = Array::new();
                for item in py_tuple.iter() {
                    let items = to_toml_impl(py, &item, inline_tables, toml_path)?;
                    match items {
                        Item::Value(value) => {
                            array.push(value);
                        }
                        Item::Table(table) => {
                            let inline_table = table.into_inline_table();
                            array.push(Value::InlineTable(inline_table));
                        }
                        _ => {
                            return Err($crate::toml_rs::TOMLEncodeError::new_err(
                                "Arrays can only contain values or inline tables",
                            ));
                        }
                    }
                }

                return $to_toml_macro!(TomlArray, array);
            }

            Err($crate::toml_rs::TOMLEncodeError::new_err(format!(
                "Cannot serialize {py_type} to TOML",
                py_type = $crate::get_type!(obj)
            )))
        }
    };
}

#[macro_export]
macro_rules! get_type {
    ($obj:expr) => {
        format!(
            "{} ({})",
            $obj.repr()
                .map(|s| s.to_string())
                .unwrap_or_else(|_| String::from("<unknown>")),
            $obj.get_type()
                .repr()
                .map(|s| s.to_string())
                .unwrap_or_else(|_| String::from("<unknown>"))
        )
    };
}
