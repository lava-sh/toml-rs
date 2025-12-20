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
            $time.second,
            $time.nanosecond / 1000,
            $tzinfo,
        )
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

#[macro_export]
macro_rules! toml_dt {
    (Date, $py_date:expr) => {
        toml::value::Date {
            year: u16::try_from($py_date.get_year())?,
            month: $py_date.get_month(),
            day: $py_date.get_day(),
        }
    };

    (Time, $py_time:expr) => {
        toml::value::Time {
            hour: $py_time.get_hour(),
            minute: $py_time.get_minute(),
            second: $py_time.get_second(),
            nanosecond: $py_time.get_microsecond() * 1000,
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
    ($var:ident, $value:expr) => {
        Ok(toml_edit::Item::Value(toml_edit::Value::$var(
            toml_edit::Formatted::new($value),
        )))
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
