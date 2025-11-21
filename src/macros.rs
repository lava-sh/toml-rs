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
    (Date, $y:expr, $m:expr, $d:expr) => {
        toml::value::Date {
            year: u16::try_from($y)?,
            month: $m,
            day: $d,
        }
    };

    (Time, $h:expr, $m:expr, $s:expr, $ns:expr) => {
        toml::value::Time {
            hour: $h,
            minute: $m,
            second: $s,
            nanosecond: $ns,
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
