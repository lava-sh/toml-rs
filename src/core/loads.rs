use pyo3::{
    prelude::*,
    types::{PyDelta, PyTzInfo},
};

pub trait TomlOffset {
    fn into_minutes(self) -> Option<i16>;
}

impl TomlOffset for toml::value::Offset {
    fn into_minutes(self) -> Option<i16> {
        match self {
            Self::Z => None,
            Self::Custom { minutes } => Some(minutes),
        }
    }
}

impl TomlOffset for toml_v1::value::Offset {
    fn into_minutes(self) -> Option<i16> {
        match self {
            Self::Z => None,
            Self::Custom { minutes } => Some(minutes),
        }
    }
}

#[inline]
pub fn create_timezone_from_offset<T: TomlOffset>(
    py: Python,
    offset: T,
) -> PyResult<Bound<PyTzInfo>> {
    const SECS_IN_DAY: i32 = 86_400;

    match offset.into_minutes() {
        None => PyTzInfo::utc(py).map(Borrowed::to_owned),
        Some(minutes) => {
            let seconds = i32::from(minutes) * 60;
            let days = seconds.div_euclid(SECS_IN_DAY);
            let seconds = seconds.rem_euclid(SECS_IN_DAY);
            let py_delta = PyDelta::new(py, days, seconds, 0, false)?;
            PyTzInfo::fixed_offset(py, py_delta)
        }
    }
}
