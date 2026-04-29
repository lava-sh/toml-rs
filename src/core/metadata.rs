use std::{borrow::Cow, ops::Range};

use memchr::{memchr, memchr_iter};
use pyo3::{
    Bound, IntoPyObjectExt, PyAny,
    prelude::*,
    types::{PyDict, PyTuple},
};

#[derive(Clone)]
pub struct KeyLoc {
    pub key: String,
    pub key_raw: String,
    pub key_line: usize,
    pub key_col: (usize, usize),
}

#[derive(Clone)]
pub struct ValueLoc {
    pub raw_span: Option<Range<usize>>,
    pub line: (usize, usize),
    pub col: (usize, usize),
}

pub fn empty_value_loc() -> ValueLoc {
    ValueLoc {
        raw_span: None,
        line: (0, 0),
        col: (0, 0),
    }
}

pub fn build_value_line(py: Python, (l1, l2): (usize, usize)) -> PyResult<Bound<PyAny>> {
    if l1 == l2 {
        Ok(l1.into_bound_py_any(py)?)
    } else {
        Ok(PyTuple::new(py, [l1, l2])?.into_any())
    }
}

pub fn build_key_col(py: Python, (c1, c2): (usize, usize)) -> PyResult<Bound<PyAny>> {
    if c1 == c2 {
        Ok(c1.into_bound_py_any(py)?)
    } else {
        Ok(PyTuple::new(py, [c1, c2])?.into_any())
    }
}

pub fn build_value_col(py: Python, (c1, c2): (usize, usize)) -> PyResult<Bound<PyAny>> {
    if c1 == c2 {
        Ok(c1.into_bound_py_any(py)?)
    } else {
        Ok(PyTuple::new(py, [c1, c2])?.into_any())
    }
}

pub fn value_raw<'a>(doc: &'a str, value: &ValueLoc) -> Cow<'a, str> {
    match &value.raw_span {
        Some(span) if span.start < span.end && span.end <= doc.len() => {
            Cow::Borrowed(&doc[span.start..span.end])
        }
        _ => Cow::Borrowed(""),
    }
}

pub fn set_key_fields(py: Python<'_>, py_dict: &Bound<'_, PyDict>, key: &KeyLoc) -> PyResult<()> {
    py_dict.set_item("key", key.key.as_str())?;
    py_dict.set_item("key_raw", key.key_raw.as_str())?;
    py_dict.set_item("key_line", key.key_line)?;
    py_dict.set_item("key_col", build_key_col(py, key.key_col)?)?;
    Ok(())
}

pub fn set_value_metadata_fields(
    py: Python<'_>,
    doc: &str,
    py_dict: &Bound<'_, PyDict>,
    value: &ValueLoc,
) -> PyResult<()> {
    let value_raw = value_raw(doc, value);
    py_dict.set_item("value_raw", value_raw.as_ref())?;
    py_dict.set_item("value_line", build_value_line(py, value.line)?)?;
    py_dict.set_item("value_col", build_value_col(py, value.col)?)?;
    Ok(())
}

pub fn build_dict<'py>(
    py: Python<'py>,
    doc: &str,
    key: Option<&KeyLoc>,
    value: &ValueLoc,
    py_value: Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py_dict = PyDict::new(py);

    if let Some(k) = key {
        set_key_fields(py, &py_dict, k)?;
    }

    py_dict.set_item("value", py_value)?;
    if value.raw_span.is_some() {
        set_value_metadata_fields(py, doc, &py_dict, value)?;
    }
    Ok(py_dict.into_any())
}

#[derive(Clone, Copy)]
pub enum NodeKind {
    RootTable,
    HeaderTable,
    InlineTable,
    Array,
    ArrayOfTables,
    ArrayOfTablesTable,
    ArrayItem,
}

pub fn raw_slice<'a>(doc: &'a str, span: &Range<usize>) -> &'a str {
    doc.get(span.start..span.end).unwrap_or("")
}

pub fn span_contains(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

pub fn table_needs_wrapper(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::InlineTable | NodeKind::ArrayOfTablesTable | NodeKind::ArrayItem
    )
}

pub fn array_has_value_metadata(kind: NodeKind) -> bool {
    !matches!(kind, NodeKind::ArrayOfTables)
}

pub fn classify_keyed_table_kind(
    value_span: &Range<usize>,
    key_span: &Range<usize>,
    parent_kind: NodeKind,
) -> NodeKind {
    if span_contains(value_span, key_span) {
        match parent_kind {
            NodeKind::RootTable
            | NodeKind::HeaderTable
            | NodeKind::InlineTable
            | NodeKind::ArrayOfTablesTable
            | NodeKind::ArrayItem => NodeKind::HeaderTable,
            NodeKind::Array | NodeKind::ArrayOfTables => NodeKind::ArrayOfTablesTable,
        }
    } else {
        NodeKind::InlineTable
    }
}

pub fn classify_array_item_table_kind(parent_kind: NodeKind) -> NodeKind {
    match parent_kind {
        NodeKind::Array => NodeKind::ArrayItem,
        NodeKind::ArrayOfTables => NodeKind::ArrayOfTablesTable,
        NodeKind::RootTable
        | NodeKind::HeaderTable
        | NodeKind::InlineTable
        | NodeKind::ArrayOfTablesTable
        | NodeKind::ArrayItem => NodeKind::HeaderTable,
    }
}

pub fn classify_keyed_array_kind(value_span: &Range<usize>, key_span: &Range<usize>) -> NodeKind {
    if span_contains(value_span, key_span) {
        NodeKind::ArrayOfTables
    } else {
        NodeKind::Array
    }
}

#[derive(Clone)]
pub struct DocIndex<'a> {
    pub doc: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> DocIndex<'a> {
    pub fn new(doc: &'a str) -> Self {
        let mut line_starts = Vec::new();
        line_starts.push(0);
        for i in memchr_iter(b'\n', doc.as_bytes()) {
            line_starts.push(i + 1);
        }
        Self { doc, line_starts }
    }

    pub fn line_col(&self, pos: usize) -> (usize, usize) {
        let pos = pos.min(self.doc.len());
        let idx = self
            .line_starts
            .binary_search(&pos)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let line = idx + 1;
        let col = pos - self.line_starts[idx] + 1;
        (line, col)
    }

    pub fn col_range_same_line(&self, start: usize, end: usize) -> (usize, usize) {
        let (_, c1) = self.line_col(start);
        let end_pos = end.saturating_sub(1).min(self.doc.len().saturating_sub(1));
        let (_, c2) = self.line_col(end_pos);
        (c1, c2)
    }

    pub fn value_line_range(&self, start: usize, end: usize) -> (usize, usize) {
        let (l1, _) = self.line_col(start);
        let end_pos = end.saturating_sub(1).min(self.doc.len().saturating_sub(1));
        let (l2, _) = self.line_col(end_pos);
        (l1, l2)
    }

    pub fn value_col_range_first_line(&self, start: usize, end: usize) -> (usize, usize) {
        let (_, c1) = self.line_col(start);
        let bytes = self.doc.as_bytes();
        let end = end.min(bytes.len());
        let nl = memchr(b'\n', &bytes[start..end]).map(|i| start + i);
        let end_pos = match nl {
            Some(nl_pos) if nl_pos > start => nl_pos - 1,
            Some(_) => start,
            None => end.saturating_sub(1).min(bytes.len().saturating_sub(1)),
        };
        let (_, c2) = self.line_col(end_pos);
        (c1, c2)
    }
}
