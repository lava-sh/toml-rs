use pyo3::types::{
    PyAnyMethods, PyBoolMethods, PyDateAccess, PyDeltaAccess, PyDictMethods, PyListMethods,
    PyStringMethods, PyTimeAccess, PyTupleMethods, PyTzInfoAccess,
};
use toml_edit::{Array, InlineTable, Item, Offset, Table, Value};

use crate::{impl_dumps, to_toml, toml_dt};

impl_dumps!(validate_inline_paths, python_to_toml, to_toml, toml_dt);
