use std::ops::Range;

use lexical_core::ParseIntegerOptions;
use num_bigint::BigInt;
use pyo3::{
    Bound, IntoPyObjectExt, PyAny,
    prelude::*,
    types::{PyDate, PyDict, PyList, PyTime},
};
use toml::{
    Spanned,
    de::{
        DeTable, DeValue,
        parser::{DeString, get_key_span},
    },
};

use crate::{
    core::{
        loads::create_timezone_from_offset,
        metadata::{
            DocIndex, KeyLoc, NodeKind, ValueLoc, array_has_value_metadata, build_dict,
            classify_array_item_table_kind, classify_keyed_array_kind, classify_keyed_table_kind,
            empty_value_loc, raw_slice, set_key_fields, set_value_metadata_fields,
            table_needs_wrapper,
        },
    },
    create_py_datetime,
    error::TomlError,
    parse_int,
    toml_rs::TOMLDecodeError,
};

fn make_key_loc(idx: &DocIndex<'_>, doc: &str, key: &Spanned<DeString<'_>>) -> KeyLoc {
    let span = get_key_span(key);
    let key_raw = raw_slice(doc, &(span.start()..span.end()));
    let (key_line, _) = idx.line_col(span.start());
    let key_col = idx.col_range_same_line(span.start(), span.end());
    KeyLoc {
        key: key.get_ref().clone().into_owned(),
        key_raw: key_raw.to_owned(),
        key_line,
        key_col,
    }
}

fn make_value_loc(idx: &DocIndex<'_>, span: &Range<usize>) -> ValueLoc {
    if span.start >= span.end {
        return empty_value_loc();
    }

    let line = idx.value_line_range(span.start, span.end);
    let col = idx.value_col_range_first_line(span.start, span.end);
    ValueLoc {
        raw_span: Some(span.clone()),
        line,
        col,
    }
}

fn scalar_to_py_obj<'py>(
    py: Python<'py>,
    doc: &str,
    kind: &DeValue<'_>,
    raw_span: Range<usize>,
) -> PyResult<Py<PyAny>> {
    match kind {
        DeValue::String(str) => str.into_py_any(py),
        DeValue::Boolean(bool) => bool.into_py_any(py),
        DeValue::Integer(int) => {
            let bytes = int.as_str().as_bytes();
            let radix = int.radix();
            let options = ParseIntegerOptions::new();
            if let Ok(i_64) = parse_int!(i64, bytes, &options, radix) {
                return i_64.into_py_any(py);
            }
            if let Some(big_int) = BigInt::parse_bytes(bytes, radix) {
                return big_int.into_py_any(py);
            }
            let mut err = TomlError::custom(
                format!(
                    "invalid integer '{}'",
                    &doc[raw_span.start..raw_span.end.min(doc.len())]
                ),
                Some(raw_span.start..raw_span.end),
            );
            err.set_input(Some(doc));
            Err(TOMLDecodeError::new_err((
                err.to_string(),
                doc.to_string(),
                raw_span.start,
            )))
        }
        DeValue::Float(float) => {
            let bytes = float.as_str().as_bytes();
            let parsed: f64 = lexical_core::parse(bytes).map_err(|err| {
                TOMLDecodeError::new_err((
                    format!("invalid float '{}': {err}", float.as_str()),
                    doc.to_string(),
                    raw_span.start,
                ))
            })?;
            parsed.into_py_any(py)
        }
        DeValue::Datetime(dt) => {
            let bound_any: Bound<'py, PyAny> = match (dt.date, dt.time, dt.offset) {
                (Some(date), Some(time), Some(offset)) => {
                    let py_tzinfo = create_timezone_from_offset(py, offset)?;
                    let tzinfo = Some(&py_tzinfo);
                    create_py_datetime!(py, date, time, tzinfo)?.into_any()
                }
                (Some(date), Some(time), None) => {
                    create_py_datetime!(py, date, time, None)?.into_any()
                }
                (Some(date), None, None) => {
                    PyDate::new(py, i32::from(date.year), date.month, date.day)?.into_any()
                }
                (None, Some(time), None) => PyTime::new(
                    py,
                    time.hour,
                    time.minute,
                    time.second.unwrap_or(0),
                    time.nanosecond.unwrap_or(0) / 1000,
                    None,
                )?
                .into_any(),
                _ => unreachable!(),
            };
            Ok(bound_any.unbind())
        }
        DeValue::Array(_) | DeValue::Table(_) => unreachable!(),
    }
}

fn build_table_nodes<'py>(
    py: Python<'py>,
    idx: &DocIndex<'_>,
    doc: &str,
    table: &DeTable<'_>,
    table_kind: NodeKind,
) -> PyResult<Bound<'py, PyAny>> {
    let py_dict = PyDict::new(py);
    for (key, value) in table {
        let key_span = get_key_span(key);
        let key_span = key_span.start()..key_span.end();
        let child_kind = match value.get_ref() {
            DeValue::Table(_) => classify_keyed_table_kind(&value.span(), &key_span, table_kind),
            DeValue::Array(_) => classify_keyed_array_kind(&value.span(), &key_span),
            DeValue::String(_)
            | DeValue::Boolean(_)
            | DeValue::Integer(_)
            | DeValue::Float(_)
            | DeValue::Datetime(_) => table_kind,
        };
        py_dict.set_item(
            key.get_ref().as_ref(),
            build_node(py, idx, doc, Some(key), value, child_kind)?,
        )?;
    }
    Ok(py_dict.into_any())
}

fn build_node<'py>(
    py: Python<'py>,
    idx: &DocIndex<'_>,
    doc: &str,
    key: Option<&Spanned<DeString<'_>>>,
    value: &Spanned<DeValue<'_>>,
    node_kind: NodeKind,
) -> PyResult<Bound<'py, PyAny>> {
    let key_loc = key.map(|key| make_key_loc(idx, doc, key));
    let value_span = value.span();
    let value_loc = make_value_loc(idx, &value_span);

    match value.get_ref() {
        DeValue::String(_)
        | DeValue::Boolean(_)
        | DeValue::Integer(_)
        | DeValue::Float(_)
        | DeValue::Datetime(_) => {
            let py_value = scalar_to_py_obj(py, doc, value.get_ref(), value_span)?;
            let py_dict = PyDict::new(py);

            if let Some(k) = key_loc.as_ref() {
                set_key_fields(py, &py_dict, k)?;
            }

            set_value_metadata_fields(py, doc, &py_dict, &value_loc)?;
            py_dict.set_item("value", py_value.bind(py))?;

            Ok(py_dict.into_any())
        }
        DeValue::Array(array) => {
            let py_list = PyList::empty(py);
            for item in array {
                let item_kind = match item.get_ref() {
                    DeValue::Table(_) => classify_array_item_table_kind(node_kind),
                    DeValue::Array(_)
                    | DeValue::String(_)
                    | DeValue::Boolean(_)
                    | DeValue::Integer(_)
                    | DeValue::Float(_)
                    | DeValue::Datetime(_) => NodeKind::ArrayItem,
                };
                py_list.append(build_node(py, idx, doc, None, item, item_kind)?)?;
            }

            let emit_value_metadata = array_has_value_metadata(node_kind);
            let array_value_loc = emit_value_metadata.then_some(&value_loc);
            let empty_array_value_loc = empty_value_loc();

            if key_loc.is_some() || array_value_loc.is_some() {
                build_dict(
                    py,
                    doc,
                    key_loc.as_ref(),
                    array_value_loc.unwrap_or(&empty_array_value_loc),
                    py_list.into_any(),
                )
            } else {
                Ok(py_list.into_any())
            }
        }
        DeValue::Table(table) => {
            let py_value_dict = build_table_nodes(py, idx, doc, table, node_kind)?;
            if table_needs_wrapper(node_kind) {
                build_dict(py, doc, key_loc.as_ref(), &value_loc, py_value_dict)
            } else {
                Ok(py_value_dict)
            }
        }
    }
}

pub fn extract_metadata<'py>(
    py: Python<'py>,
    table: &Spanned<DeTable<'_>>,
    doc: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let idx = DocIndex::new(doc);

    let py_dict = PyDict::new(py);
    py_dict.set_item(
        "nodes",
        build_table_nodes(py, &idx, doc, table.get_ref(), NodeKind::RootTable)?,
    )?;
    Ok(py_dict.into_any())
}

pub fn to_python<'py>(
    py: Python<'py>,
    value: &DeValue<'_>,
    span: Range<usize>,
    doc: &str,
) -> PyResult<Bound<'py, PyAny>> {
    match value {
        DeValue::String(str) => str.into_bound_py_any(py),
        DeValue::Boolean(bool) => bool.into_bound_py_any(py),
        DeValue::Integer(int) => {
            let bytes = int.as_str().as_bytes();
            let radix = int.radix();
            let options = ParseIntegerOptions::new();

            if let Ok(i_64) = parse_int!(i64, bytes, &options, radix) {
                return i_64.into_bound_py_any(py);
            }

            if let Some(big_int) = BigInt::parse_bytes(bytes, radix) {
                return big_int.into_bound_py_any(py);
            }

            let mut err = TomlError::custom(
                format!(
                    "invalid integer '{}'",
                    &doc[span.start..span.end.min(doc.len())]
                ),
                Some(span.start..span.end),
            );
            err.set_input(Some(doc));

            Err(TOMLDecodeError::new_err((
                err.to_string(),
                doc.to_string(),
                span.start,
            )))
        }
        DeValue::Float(float) => {
            let bytes = float.as_str().as_bytes();
            let parsed: f64 = lexical_core::parse(bytes).map_err(|err| {
                TOMLDecodeError::new_err((
                    format!("invalid float '{}': {err}", float.as_str()),
                    doc.to_string(),
                    span.start,
                ))
            })?;
            parsed.into_bound_py_any(py)
        }
        DeValue::Datetime(dt) => match (dt.date, dt.time, dt.offset) {
            (Some(date), Some(time), Some(offset)) => {
                let py_tzinfo = create_timezone_from_offset(py, offset)?;
                let tzinfo = Some(&py_tzinfo);
                Ok(create_py_datetime!(py, date, time, tzinfo)?.into_any())
            }
            (Some(date), Some(time), None) => {
                Ok(create_py_datetime!(py, date, time, None)?.into_any())
            }
            (Some(date), None, None) => {
                Ok(PyDate::new(py, i32::from(date.year), date.month, date.day)?.into_any())
            }
            (None, Some(time), None) => Ok(PyTime::new(
                py,
                time.hour,
                time.minute,
                time.second.unwrap_or(0),
                time.nanosecond.unwrap_or(0) / 1000,
                None,
            )?
            .into_any()),
            _ => unreachable!(),
        },
        DeValue::Array(array) => {
            let py_list = PyList::empty(py);
            for item in array {
                py_list.append(to_python(py, item.get_ref(), item.span(), doc)?)?;
            }
            Ok(py_list.into_any())
        }
        DeValue::Table(table) => {
            let py_dict = PyDict::new(py);
            for (k, v) in table {
                let val = to_python(py, v.get_ref(), v.span(), doc)?;
                py_dict.set_item(k.get_ref().as_ref(), val)?;
            }
            Ok(py_dict.into_any())
        }
    }
}
