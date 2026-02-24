use pyo3::types::{
    PyAnyMethods, PyBoolMethods, PyDateAccess, PyDeltaAccess, PyDictMethods, PyListMethods,
    PyStringMethods, PyTimeAccess, PyTzInfoAccess,
};
use toml_edit_v1::{Array, InlineTable, Item, Offset, Table, Value};

use crate::{impl_dumps, to_toml_v1, toml_dt_v1};

impl_dumps!(
    validate_inline_paths_v1,
    python_to_toml_v1,
    to_toml_v1,
    toml_dt_v1
);
