#[macro_export]
macro_rules! impl_to_python {
    (
        $create_py_datetime:ident,
        $time_second:expr,
        $time_microsecond:expr
    ) => {
        fn to_python<'py>(
            py: pyo3::Python<'py>,
            de_value: &Spanned<DeValue<'_>>,
            parse_float: &pyo3::Bound<'py, pyo3::PyAny>,
            doc: &str,
        ) -> pyo3::PyResult<pyo3::Bound<'py, pyo3::PyAny>> {
            let value = de_value.get_ref();
            let span = de_value.span();

            match value {
                DeValue::String(str) => pyo3::IntoPyObjectExt::into_bound_py_any(str, py),
                DeValue::Integer(int) => {
                    let bytes = int.as_str().as_bytes();
                    let radix = int.radix();

                    let options = lexical_core::ParseIntegerOptions::new();

                    if let Ok(i_64) = $crate::parse_int!(i64, bytes, &options, radix) {
                        return pyo3::IntoPyObjectExt::into_bound_py_any(i_64, py);
                    }

                    if let Some(big_int) = num_bigint::BigInt::parse_bytes(bytes, radix) {
                        return pyo3::IntoPyObjectExt::into_bound_py_any(big_int, py);
                    }

                    let mut err = $crate::error::TomlError::custom(
                        format!(
                            "invalid integer '{}'",
                            &doc[span.start..span.end.min(doc.len())]
                        ),
                        Some(span.start..span.end),
                    );
                    err.set_input(Some(doc));

                    Err($crate::toml_rs::TOMLDecodeError::new_err((
                        err.to_string(),
                        doc.to_string(),
                        span.start,
                    )))
                }
                DeValue::Float(float) => {
                    let float_str = float.as_str();

                    let py_call = parse_float.call1((float_str,))?;

                    if py_call.is_exact_instance_of::<pyo3::types::PyDict>()
                        || py_call.is_exact_instance_of::<pyo3::types::PyList>()
                    {
                        return Err(pyo3::exceptions::PyValueError::new_err(
                            "parse_float must not return dicts or lists",
                        ));
                    }

                    Ok(py_call)
                }
                DeValue::Boolean(bool) => pyo3::IntoPyObjectExt::into_bound_py_any(bool, py),
                DeValue::Datetime(datetime) => {
                    match (datetime.date, datetime.time, datetime.offset) {
                        (Some(date), Some(time), Some(offset)) => {
                            let tzinfo = Some(&$crate::core::loads::create_timezone_from_offset(
                                py, offset,
                            )?);
                            Ok($create_py_datetime!(py, date, time, tzinfo)?.into_any())
                        }
                        (Some(date), Some(time), None) => {
                            Ok($create_py_datetime!(py, date, time, None)?.into_any())
                        }
                        (Some(date), None, None) => {
                            let py_date = pyo3::types::PyDate::new(
                                py,
                                i32::from(date.year),
                                date.month,
                                date.day,
                            )?;
                            Ok(py_date.into_any())
                        }
                        (None, Some(time), None) => {
                            let py_time = pyo3::types::PyTime::new(
                                py,
                                time.hour,
                                time.minute,
                                $time_second(&time),
                                $time_microsecond(&time),
                                None,
                            )?;
                            Ok(py_time.into_any())
                        }
                        _ => unreachable!(),
                    }
                }
                DeValue::Array(array) => {
                    if array.is_empty() {
                        return Ok(pyo3::types::PyList::empty(py).into_any());
                    }

                    let py_list = pyo3::types::PyList::empty(py);
                    for item in array {
                        py_list.append(to_python(py, item, parse_float, doc)?)?;
                    }

                    Ok(py_list.into_any())
                }
                DeValue::Table(table) => {
                    if table.is_empty() {
                        return Ok(pyo3::types::PyDict::new(py).into_any());
                    }

                    let py_dict = pyo3::types::PyDict::new(py);
                    for (k, v) in table {
                        let key = k.get_ref().clone().into_owned();
                        let value = to_python(py, v, parse_float, doc)?;
                        py_dict.set_item(key, value)?;
                    }

                    Ok(py_dict.into_any())
                }
            }
        }
    };
}

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

            Err($crate::toml_rs::TOMLEncodeError::new_err(format!(
                "Cannot serialize {py_type} to TOML",
                py_type = $crate::get_type!(obj)
            )))
        }
    };
}

#[macro_export]
macro_rules! impl_pretty {
    ($pretty_name:ident) => {
        pub(crate) struct $pretty_name {
            in_value: bool,
            format_tables: bool,
        }

        impl $pretty_name {
            pub(crate) fn new(format_tables: bool) -> Self {
                Self {
                    in_value: false,
                    format_tables,
                }
            }
        }

        fn make_item(node: &mut Item) {
            *node = std::mem::take(node)
                .into_table()
                .map_or_else(|i| i, Item::Table)
                .into_array_of_tables()
                .map_or_else(|i| i, Item::ArrayOfTables);
        }

        impl visit_mut::VisitMut for $pretty_name {
            fn visit_document_mut(&mut self, node: &mut DocumentMut) {
                visit_mut::visit_document_mut(self, node);
            }

            fn visit_item_mut(&mut self, node: &mut Item) {
                if !self.in_value && self.format_tables {
                    make_item(node);
                }

                visit_mut::visit_item_mut(self, node);
            }

            fn visit_table_mut(&mut self, node: &mut Table) {
                node.decor_mut().clear();

                // Empty tables could be semantically meaningful, so make sure they are not implicit
                if !node.is_empty() {
                    node.set_implicit(true);
                }

                visit_mut::visit_table_mut(self, node);
            }

            fn visit_array_mut(&mut self, node: &mut Array) {
                visit_mut::visit_array_mut(self, node);

                if (0..=1).contains(&node.len()) {
                    node.set_trailing("");
                    node.set_trailing_comma(false);
                } else {
                    for item in node.iter_mut() {
                        item.decor_mut().set_prefix("\n    ");
                    }
                    node.set_trailing("\n");
                    node.set_trailing_comma(true);
                }
            }

            fn visit_value_mut(&mut self, node: &mut Value) {
                node.decor_mut().clear();

                let old_in_value = self.in_value;
                self.in_value = true;
                visit_mut::visit_value_mut(self, node);
                self.in_value = old_in_value;
            }
        }
    };
}
