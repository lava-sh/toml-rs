#[macro_export]
macro_rules! create_py_datetime {
    ($py:expr, $date:expr, $time:expr, $tzinfo:expr) => {
        PyDateTime::new(
            $py,
            $date.year as i32,
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
macro_rules! get_repr {
    ($obj:expr) => {
        $obj.repr()
            .map(|s| s.to_string())
            .unwrap_or_else(|_| "<unknown>".to_string())
    };
}
