// https://github.com/toml-rs/toml/blob/v0.25.3/crates/toml_edit/src/ser/pretty.rs
use toml_edit::{Array, DocumentMut, Item, Table, Value, visit_mut};

use crate::impl_pretty;

impl_pretty!(Pretty);
