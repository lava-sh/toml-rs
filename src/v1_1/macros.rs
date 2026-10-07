#[macro_export]
macro_rules! create_py_datetime {
    ($py:expr, $date:expr, $time:expr, $tzinfo:expr) => {
        pyo3::types::PyDateTime::new(
            $py,
            i32::from($date.year),
            $date.month,
            $date.day,
            $time.hour,
            $time.minute,
            $time.second.unwrap_or(0),
            $time.nanosecond.unwrap_or(0) / 1000,
            $tzinfo,
        )
    };
}

#[macro_export]
macro_rules! toml_dt {
    (Date, $py_date:expr) => {
        toml::value::Date {
            year: u16::try_from(cfg_select! {
                not(Py_LIMITED_API) => $py_date.get_year(),
                Py_LIMITED_API => $py_date
                    .getattr(pyo3::intern!($py_date.py(), "year"))?
                    .extract::<i32>()?,
            })?,
            month: cfg_select! {
                not(Py_LIMITED_API) => $py_date.get_month(),
                Py_LIMITED_API => $py_date
                    .getattr(pyo3::intern!($py_date.py(), "month"))?
                    .extract::<u8>()?,
            },
            day: cfg_select! {
                not(Py_LIMITED_API) => $py_date.get_day(),
                Py_LIMITED_API => $py_date
                    .getattr(pyo3::intern!($py_date.py(), "day"))?
                    .extract::<u8>()?,
            },
        }
    };

    (Time, $py_time:expr) => {
        toml::value::Time {
            hour: cfg_select! {
                not(Py_LIMITED_API) => $py_time.get_hour(),
                Py_LIMITED_API => $py_time
                    .getattr(pyo3::intern!($py_time.py(), "hour"))?
                    .extract::<u8>()?,
            },
            minute: cfg_select! {
                not(Py_LIMITED_API) => $py_time.get_minute(),
                Py_LIMITED_API => $py_time
                    .getattr(pyo3::intern!($py_time.py(), "minute"))?
                    .extract::<u8>()?,
            },
            second: Some(cfg_select! {
                not(Py_LIMITED_API) => $py_time.get_second(),
                Py_LIMITED_API => $py_time
                    .getattr(pyo3::intern!($py_time.py(), "second"))?
                    .extract::<u8>()?,
            }),
            nanosecond: Some(cfg_select! {
                not(Py_LIMITED_API) => $py_time.get_microsecond() * 1000,
                Py_LIMITED_API => {
                    $py_time
                        .getattr(pyo3::intern!($py_time.py(), "microsecond"))?
                        .extract::<u32>()?
                        * 1000
                }
            }),
        }
    };

    (Datetime, $date:expr, $time:expr, $offset:expr) => {
        toml::value::Datetime {
            date: $date,
            time: $time,
            offset: $offset,
        }
    };
}

#[macro_export]
macro_rules! to_toml {
    (TomlTable, $value:expr) => {
        Ok(toml_edit::Item::Table($value))
    };
    (TomlArray, $value:expr) => {
        Ok(toml_edit::Item::Value(toml_edit::Value::Array($value)))
    };
    (TomlInlineTable, $value:expr) => {
        Ok(toml_edit::Item::Value(toml_edit::Value::InlineTable(
            $value,
        )))
    };
    (BigNum, $value:expr) => {{
        let num = toml_edit::BigNum::new($value);
        Ok(toml_edit::Item::Value(toml_edit::Value::BigNum(
            toml_edit::Formatted::new(num),
        )))
    }};
    ($var:ident, $value:expr) => {
        Ok(toml_edit::Item::Value(toml_edit::Value::$var(
            toml_edit::Formatted::new($value),
        )))
    };
}
