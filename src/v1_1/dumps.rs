use pyo3::types::{
    PyAnyMethods, PyBoolMethods, PyListMethods, PyStringMethods, PyTupleMethods, PyTzInfoAccess,
};
#[cfg(not(Py_LIMITED_API))]
use pyo3::types::{PyDateAccess, PyDeltaAccess, PyTimeAccess};
use toml_edit::{Array, InlineTable, Item, Offset, Table, Value};

use crate::{impl_dumps, to_toml, toml_dt};

impl_dumps!(validate_inline_paths, python_to_toml, to_toml, toml_dt);
