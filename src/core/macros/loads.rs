#[macro_export]
macro_rules! impl_loads {
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
                            let py_tzinfo =
                                $crate::core::loads::create_timezone_from_offset(py, offset)?;
                            let tzinfo = Some(&py_tzinfo);
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
macro_rules! parse_int {
    ($int:ty, $bytes:expr, $options:expr, $radix:expr) => {
        match $radix {
            2 => lexical_core::parse_with_options::<
                $int,
                { lexical_core::NumberFormatBuilder::from_radix(2) },
            >($bytes, $options),
            8 => lexical_core::parse_with_options::<
                $int,
                { lexical_core::NumberFormatBuilder::from_radix(8) },
            >($bytes, $options),
            10 => lexical_core::parse_with_options::<
                $int,
                { lexical_core::NumberFormatBuilder::from_radix(10) },
            >($bytes, $options),
            16 => lexical_core::parse_with_options::<
                $int,
                { lexical_core::NumberFormatBuilder::from_radix(16) },
            >($bytes, $options),
            _ => unreachable!(),
        }
    };
}
