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
            fn get_decimal_type(
                py: pyo3::Python<'_>,
            ) -> pyo3::PyResult<&pyo3::Bound<'_, pyo3::types::PyType>> {
                static DECIMAL_TYPE: pyo3::sync::PyOnceLock<pyo3::Py<pyo3::types::PyType>> =
                    pyo3::sync::PyOnceLock::new();

                DECIMAL_TYPE.import(py, "decimal", "Decimal")
            }

            fn get_mapping_type(
                py: pyo3::Python<'_>,
            ) -> pyo3::PyResult<&pyo3::Bound<'_, pyo3::types::PyType>> {
                static MAPPING_TYPE: pyo3::sync::PyOnceLock<pyo3::Py<pyo3::types::PyType>> =
                    pyo3::sync::PyOnceLock::new();

                MAPPING_TYPE.import(py, "collections.abc", "Mapping")
            }

            fn get_isinstance_func(
                py: pyo3::Python<'_>,
            ) -> pyo3::PyResult<&pyo3::Bound<'_, pyo3::PyAny>> {
                static ISINSTANCE_FUNC: pyo3::sync::PyOnceLock<pyo3::Py<pyo3::PyAny>> =
                    pyo3::sync::PyOnceLock::new();

                ISINSTANCE_FUNC
                    .get_or_try_init(py, || {
                        py.import("builtins")?
                            .getattr("isinstance")
                            .map(|func| func.unbind())
                    })
                    .map(|func| func.bind(py))
            }

            fn normalize_decimal_str(value: &str) -> std::borrow::Cow<'_, str> {
                let bytes = value.as_bytes();
                let mut start = 0;
                let mut end = bytes.len();

                // SAFETY: `start < end <= bytes.len()`, so `start` is in bounds.
                while start < end && unsafe { bytes.get_unchecked(start) }.is_ascii_whitespace() {
                    start += 1;
                }
                // SAFETY: `start < end <= bytes.len()`, so `end - 1` is in bounds.
                while start < end && unsafe { bytes.get_unchecked(end - 1) }.is_ascii_whitespace()
                {
                    end -= 1;
                }

                // SAFETY: `start` and `end` are advanced only while staying within `bytes.len()`.
                let trimmed = unsafe { value.get_unchecked(start..end) };
                let bytes = trimmed.as_bytes();

                let (neg, rest, sign_len) = match bytes {
                    [b'-', rest @ ..] => (true, rest, 1),
                    [b'+', rest @ ..] => (false, rest, 1),
                    _ => (false, bytes, 0),
                };

                if rest.len() == 3 {
                    // SAFETY: `rest.len() == 3`, so indices `0..3` are in bounds.
                    let (a, b, c) = unsafe {
                        (
                            *rest.get_unchecked(0) | 0x20,
                            *rest.get_unchecked(1) | 0x20,
                            *rest.get_unchecked(2) | 0x20,
                        )
                    };

                    if (a, b, c) == (b'n', b'a', b'n') {
                        return std::borrow::Cow::Borrowed("nan");
                    }
                }

                if rest.len() == 8 {
                    let inf = b"infinity";
                    let mut matches = true;

                    for i in 0..8 {
                        // SAFETY: `rest.len() == 8`, so `i` in `0..8` is in bounds.
                        if unsafe { *rest.get_unchecked(i) | 0x20 } != inf[i] {
                            matches = false;
                            break;
                        }
                    }

                    if matches {
                        return if neg {
                            std::borrow::Cow::Borrowed("-inf")
                        } else {
                            std::borrow::Cow::Borrowed("inf")
                        };
                    }
                }

                let mut has_dot = false;
                let mut has_exp = false;
                let mut has_upper_exp = false;

                for i in sign_len..bytes.len() {
                    // SAFETY: loop bounds guarantee `i < bytes.len()`.
                    match unsafe { *bytes.get_unchecked(i) } {
                        b'.' => has_dot = true,
                        b'e' => has_exp = true,
                        b'E' => {
                            has_exp = true;
                            has_upper_exp = true;
                        }
                        _ => {}
                    }
                }

                if !has_dot && !has_exp {
                    let mut normalized = String::with_capacity(trimmed.len() + 2);
                    normalized.push_str(trimmed);
                    normalized.push_str(".0");
                    return std::borrow::Cow::Owned(normalized);
                }

                if has_upper_exp {
                    let mut normalized = trimmed.as_bytes().to_vec();
                    for byte in &mut normalized {
                        if *byte == b'E' {
                            *byte = b'e';
                        }
                    }

                    // SAFETY: the source is valid UTF-8 ASCII and we only replace `E` with `e`.
                    return std::borrow::Cow::Owned(unsafe {
                        String::from_utf8_unchecked(normalized)
                    });
                }

                std::borrow::Cow::Borrowed(trimmed)
            }

            fn mapping_to_toml_impl<'py>(
                py: pyo3::Python<'py>,
                obj: &pyo3::Bound<'py, pyo3::PyAny>,
                inline_tables: Option<&rustc_hash::FxHashSet<String>>,
                toml_path: &mut smallvec::SmallVec<String, 32>,
            ) -> pyo3::PyResult<Item> {
                let items = obj.call_method0(pyo3::intern!(py, "items"))?;
                if items.len()? == 0 {
                    return $to_toml_macro!(TomlTable, Table::new());
                }

                let inline = inline_tables.is_some_and(|set| set.contains(&toml_path.join(".")));

                if inline {
                    let mut inline_table = InlineTable::new();
                    for item in items.try_iter()? {
                        let py_tuple = item?.cast_into::<pyo3::types::PyTuple>()?;
                        let py_key = py_tuple.get_item(0)?;
                        let key = py_key
                            .clone()
                            .cast_into::<pyo3::types::PyString>()
                            .map_err(|_| {
                                $crate::toml_rs::TOMLEncodeError::new_err(format!(
                                    "TOML table keys must be strings, got {py_type}",
                                    py_type = $crate::get_type!(py_key)
                                ))
                            })?;
                        let value = py_tuple.get_item(1)?;
                        let key_str = key.to_str()?;

                        toml_path.push(key_str.to_owned());
                        let item = to_toml_impl(py, &value, inline_tables, toml_path)?;
                        toml_path.pop();

                        if let Item::Value(val) = item {
                            inline_table.insert(key_str, val);
                        } else {
                            return Err($crate::toml_rs::TOMLEncodeError::new_err(
                                "Inline tables can only contain values, not nested tables",
                            ));
                        }
                    }

                    return $to_toml_macro!(TomlInlineTable, inline_table);
                }

                let mut table = Table::new();
                for item in items.try_iter()? {
                    let py_tuple = item?.cast_into::<pyo3::types::PyTuple>()?;
                    let py_key = py_tuple.get_item(0)?;
                    let key = py_key
                        .clone()
                        .cast_into::<pyo3::types::PyString>()
                        .map_err(|_| {
                            $crate::toml_rs::TOMLEncodeError::new_err(format!(
                                "TOML table keys must be strings, got {py_type}",
                                py_type = $crate::get_type!(py_key)
                            ))
                        })?;
                    let value = py_tuple.get_item(1)?;
                    let key_str = key.to_str()?;

                    toml_path.push(key_str.to_owned());
                    let item = to_toml_impl(py, &value, inline_tables, toml_path)?;
                    toml_path.pop();

                    table.insert(key_str, item);
                }
                $to_toml_macro!(TomlTable, table)
            }

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

            if get_isinstance_func(py)?
                .call1((obj, get_decimal_type(py)?))?
                .is_truthy()?
            {
                let py_str = obj.str()?;
                let normalized = normalize_decimal_str(py_str.to_str()?);
                return $to_toml_macro!(BigNum, normalized.as_ref());
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
                return mapping_to_toml_impl(py, dict.as_any(), inline_tables, toml_path);
            }

            if get_isinstance_func(py)?
                .call1((obj, get_mapping_type(py)?))?
                .is_truthy()?
            {
                return mapping_to_toml_impl(py, obj, inline_tables, toml_path);
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
