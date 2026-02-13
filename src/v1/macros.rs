#[macro_export]
macro_rules! create_py_datetime_v1 {
    ($py:expr, $date:expr, $time:expr, $tzinfo:expr) => {
        pyo3::types::PyDateTime::new(
            $py,
            i32::from($date.year),
            $date.month,
            $date.day,
            $time.hour,
            $time.minute,
            $time.second,
            $time.nanosecond / 1000,
            $tzinfo,
        )
    };
}

#[macro_export]
macro_rules! toml_dt_v1 {
    (Date, $py_date:expr) => {
        toml_v1::value::Date {
            year: u16::try_from($py_date.get_year())?,
            month: $py_date.get_month(),
            day: $py_date.get_day(),
        }
    };

    (Time, $py_time:expr) => {
        toml_v1::value::Time {
            hour: $py_time.get_hour(),
            minute: $py_time.get_minute(),
            second: $py_time.get_second(),
            nanosecond: $py_time.get_microsecond() * 1000,
        }
    };

    (Datetime, $date:expr, $time:expr, $offset:expr) => {
        toml_v1::value::Datetime {
            date: $date,
            time: $time,
            offset: $offset,
        }
    };
}

#[macro_export]
macro_rules! to_toml_v1 {
    (TomlTable, $value:expr) => {
        Ok(toml_edit_v1::Item::Table($value))
    };
    (TomlArray, $value:expr) => {
        Ok(toml_edit_v1::Item::Value(toml_edit_v1::Value::Array(
            $value,
        )))
    };
    (TomlInlineTable, $value:expr) => {
        Ok(toml_edit_v1::Item::Value(toml_edit_v1::Value::InlineTable(
            $value,
        )))
    };
    (BigNum, $value:expr) => {{
        let num = toml_edit_v1::BigNum::new($value);
        Ok(toml_edit_v1::Item::Value(toml_edit_v1::Value::BigNum(
            toml_edit_v1::Formatted::new(num),
        )))
    }};
    ($var:ident, $value:expr) => {
        Ok(toml_edit_v1::Item::Value(toml_edit_v1::Value::$var(
            toml_edit_v1::Formatted::new($value),
        )))
    };
}
