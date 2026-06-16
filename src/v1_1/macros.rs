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
            #[cfg(not(Py_LIMITED_API))]
            year: u16::try_from($py_date.get_year())?,
            #[cfg(Py_LIMITED_API)]
            year: u16::try_from($py_date.getattr("year")?.extract()?)?,
            #[cfg(not(Py_LIMITED_API))]
            month: $py_date.get_month(),
            #[cfg(Py_LIMITED_API)]
            month: $py_date.getattr("month")?.extract()?,
            #[cfg(not(Py_LIMITED_API))]
            day: $py_date.get_day(),
            #[cfg(Py_LIMITED_API)]
            day: $py_date.getattr("day")?.extract()?,
        }
    };

    (Time, $py_time:expr) => {
        toml::value::Time {
            #[cfg(not(Py_LIMITED_API))]
            hour: $py_time.get_hour(),
            #[cfg(Py_LIMITED_API)]
            hour: $py_time.getattr("hour")?.extract()?,
            #[cfg(not(Py_LIMITED_API))]
            minute: $py_time.get_minute(),
            #[cfg(Py_LIMITED_API)]
            minute: $py_time.getattr("minute")?.extract()?,
            #[cfg(not(Py_LIMITED_API))]
            second: Some($py_time.get_second()),
            #[cfg(Py_LIMITED_API)]
            second: Some($py_time.getattr("second")?.extract()?),
            #[cfg(not(Py_LIMITED_API))]
            nanosecond: Some($py_time.get_microsecond() * 1000),
            #[cfg(Py_LIMITED_API)]
            nanosecond: Some($py_time.getattr("microsecond")?.extract::<u32>()? * 1000),
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
