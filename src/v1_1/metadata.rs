use std::{borrow::Cow, ops::Range};

use lexical_core::ParseIntegerOptions;
use memchr::{memchr, memchr_iter, memrchr};
use num_bigint::BigInt;
use pyo3::{
    Bound, IntoPyObjectExt, PyAny,
    prelude::*,
    types::{PyDate, PyDict, PyList, PyTime, PyTuple},
};
use toml::{
    Spanned,
    de::{DeTable, DeValue},
    value::Datetime,
};
use toml_parser::{
    ErrorSink, Source, Span, decoder,
    decoder::ScalarKind,
    parser::{Event, EventKind, EventReceiver, parse_document},
};

use crate::{
    create_py_datetime, error::TomlError, parse_int, toml_rs::TOMLDecodeError,
    v1_1::loads::create_timezone_from_offset,
};

#[derive(Clone)]
struct DocIndex<'a> {
    doc: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> DocIndex<'a> {
    fn new(doc: &'a str) -> Self {
        let mut line_starts = Vec::new();
        line_starts.push(0);
        for i in memchr_iter(b'\n', doc.as_bytes()) {
            line_starts.push(i + 1);
        }
        Self { doc, line_starts }
    }

    fn line_col(&self, pos: usize) -> (usize, usize) {
        let pos = pos.min(self.doc.len());
        let idx = self
            .line_starts
            .binary_search(&pos)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let line = idx + 1;
        let col = pos - self.line_starts[idx] + 1;
        (line, col)
    }

    fn slice(&self, start: usize, end: usize) -> &str {
        if start < end && end <= self.doc.len() {
            &self.doc[start..end]
        } else {
            ""
        }
    }

    fn find_open_back_to_line(&self, ch: u8, pos: usize) -> usize {
        let bytes = self.doc.as_bytes();
        if bytes.is_empty() {
            return 0;
        }
        let pos = pos.min(bytes.len().saturating_sub(1));
        let line_start = match memrchr(b'\n', &bytes[..=pos]) {
            Some(i) => i + 1,
            None => 0,
        };
        match memrchr(ch, &bytes[line_start..=pos]) {
            Some(i) => line_start + i,
            None => line_start,
        }
    }

    fn find_close_forward_to_line(&self, ch: u8, pos: usize) -> usize {
        let bytes = self.doc.as_bytes();
        let pos = pos.min(bytes.len());
        let line_end = match memchr(b'\n', &bytes[pos..]) {
            Some(i) => pos + i,
            None => bytes.len(),
        };
        match memchr(ch, &bytes[pos..line_end]) {
            Some(i) => pos + i + 1,
            None => line_end,
        }
    }

    fn col_range_same_line(&self, start: usize, end: usize) -> (usize, usize) {
        let (_, c1) = self.line_col(start);
        let end_pos = end.saturating_sub(1).min(self.doc.len().saturating_sub(1));
        let (_, c2) = self.line_col(end_pos);
        (c1, c2)
    }

    fn value_line_range(&self, start: usize, end: usize) -> (usize, usize) {
        let (l1, _) = self.line_col(start);
        let end_pos = end.saturating_sub(1).min(self.doc.len().saturating_sub(1));
        let (l2, _) = self.line_col(end_pos);
        (l1, l2)
    }

    fn value_col_range_first_line(&self, start: usize, end: usize) -> (usize, usize) {
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

#[derive(Clone)]
struct KeyLoc {
    key: String,
    key_raw: String,
    key_line: usize,
    key_col: (usize, usize),
}

struct ValueLoc {
    value_raw: String,
    value_line: (usize, usize),
    value_col: (usize, usize),
}

enum MetaNode {
    Root {
        children: rustc_hash::FxHashMap<String, MetaNode>,
    },
    Scalar {
        key: Option<KeyLoc>,
        value: ValueLoc,
        py_value: Py<PyAny>,
    },
    Table {
        key: Option<KeyLoc>,
        value: ValueLoc,
        children: rustc_hash::FxHashMap<String, MetaNode>,
    },
    Array {
        key: Option<KeyLoc>,
        value: ValueLoc,
        items: Vec<MetaNode>,
    },
}

#[derive(Clone)]
enum PathSeg {
    Key(String),
    Index(usize),
}

struct PendingKey {
    path: Vec<PathSeg>,
    key_loc: KeyLoc,
}

struct InlineCtx {
    path: Vec<PathSeg>,
    start: usize,
    key_loc: Option<KeyLoc>,
    children: rustc_hash::FxHashMap<String, MetaNode>,
}

struct ArrayCtx {
    path: Vec<PathSeg>,
    start: usize,
    key_loc: Option<KeyLoc>,
    items: Vec<MetaNode>,
}

struct Collector<'a, 'py> {
    py: Python<'py>,
    source: &'a Source<'a>,
    idx: DocIndex<'a>,
    doc: &'a str,

    root: MetaNode,
    current_table_path: Vec<PathSeg>,

    parsing_table_header: bool,
    header_is_array: bool,
    header_keys: Vec<String>,
    header_key_locs: Vec<KeyLoc>,
    header_start: usize,
    header_end: usize,
    aot_counters: rustc_hash::FxHashMap<String, usize>,

    pending: Option<PendingKey>,
    inline_pending: Option<PendingKey>,

    inline_stack: Vec<InlineCtx>,
    array_stack: Vec<ArrayCtx>,
}

impl<'a, 'py> Collector<'a, 'py> {
    fn new(py: Python<'py>, source: &'a Source<'a>, doc: &'a str) -> Self {
        Self {
            py,
            source,
            idx: DocIndex::new(doc),
            doc,
            root: MetaNode::Root {
                children: rustc_hash::FxHashMap::default(),
            },
            current_table_path: Vec::new(),
            parsing_table_header: false,
            header_is_array: false,
            header_keys: Vec::new(),
            header_key_locs: Vec::new(),
            header_start: 0,
            header_end: 0,
            aot_counters: rustc_hash::FxHashMap::default(),
            pending: None,
            inline_pending: None,
            inline_stack: Vec::new(),
            array_stack: Vec::new(),
        }
    }

    fn is_array_header_at(&self, span_start: usize) -> bool {
        let b = self.doc.as_bytes();
        span_start >= 2 && b[span_start - 1] == b'[' && b[span_start - 2] == b'['
    }

    fn header_key_string(keys: &[String]) -> String {
        keys.join(".")
    }

    fn make_key_loc(&self, key: String, key_raw: String, start: usize, end: usize) -> KeyLoc {
        let (line, _) = self.idx.line_col(start);
        let col = self.idx.col_range_same_line(start, end);
        KeyLoc {
            key,
            key_raw,
            key_line: line,
            key_col: col,
        }
    }

    fn make_value_loc(&self, start: usize, end: usize, raw: String) -> ValueLoc {
        let value_line = self.idx.value_line_range(start, end);
        let value_col = self.idx.value_col_range_first_line(start, end);
        ValueLoc {
            value_raw: raw,
            value_line,
            value_col,
        }
    }

    fn pending_for_key(&self, key: &str, key_loc: KeyLoc) -> PendingKey {
        let mut path = self.current_table_path.clone();
        path.push(PathSeg::Key(key.to_string()));
        PendingKey { path, key_loc }
    }

    fn pending_for_inline_child(parent: &[PathSeg], key: &str, key_loc: KeyLoc) -> PendingKey {
        let mut path = parent.to_vec();
        path.push(PathSeg::Key(key.to_string()));
        PendingKey { path, key_loc }
    }

    fn insert_node_at(&mut self, path: &[PathSeg], node: MetaNode) {
        fn ensure_root_children(
            cur: &mut MetaNode,
        ) -> &mut rustc_hash::FxHashMap<String, MetaNode> {
            match cur {
                MetaNode::Root { children } => children,
                MetaNode::Table { children, .. } => children,
                _ => {
                    *cur = MetaNode::Table {
                        key: None,
                        value: ValueLoc {
                            value_raw: String::new(),
                            value_line: (0, 0),
                            value_col: (0, 0),
                        },
                        children: rustc_hash::FxHashMap::default(),
                    };
                    match cur {
                        MetaNode::Table { children, .. } => children,
                        _ => unreachable!(),
                    }
                }
            }
        }

        fn ensure_array_items(cur: &mut MetaNode) -> &mut Vec<MetaNode> {
            match cur {
                MetaNode::Array { items, .. } => items,
                _ => {
                    *cur = MetaNode::Array {
                        key: None,
                        value: ValueLoc {
                            value_raw: String::new(),
                            value_line: (0, 0),
                            value_col: (0, 0),
                        },
                        items: Vec::new(),
                    };
                    match cur {
                        MetaNode::Array { items, .. } => items,
                        _ => unreachable!(),
                    }
                }
            }
        }

        let mut cur = &mut self.root;
        for (i, seg) in path.iter().enumerate() {
            let is_last = i + 1 == path.len();
            match seg {
                PathSeg::Key(k) => {
                    let children = ensure_root_children(cur);
                    if is_last {
                        children.insert(k.clone(), node);
                        return;
                    }
                    cur = children
                        .entry(k.clone())
                        .or_insert_with(|| MetaNode::Table {
                            key: None,
                            value: ValueLoc {
                                value_raw: String::new(),
                                value_line: (0, 0),
                                value_col: (0, 0),
                            },
                            children: rustc_hash::FxHashMap::default(),
                        });
                }
                PathSeg::Index(idx) => {
                    let items = ensure_array_items(cur);
                    while items.len() <= *idx {
                        items.push(MetaNode::Table {
                            key: None,
                            value: ValueLoc {
                                value_raw: String::new(),
                                value_line: (0, 0),
                                value_col: (0, 0),
                            },
                            children: rustc_hash::FxHashMap::default(),
                        });
                    }
                    if is_last {
                        items[*idx] = node;
                        return;
                    }
                    cur = &mut items[*idx];
                }
            }
        }
    }

    fn current_array_item_path(&self) -> Option<Vec<PathSeg>> {
        let ctx = self.array_stack.last()?;
        let idx = ctx.items.len();
        let mut p = ctx.path.clone();
        p.push(PathSeg::Index(idx));
        Some(p)
    }

    fn start_array_ctx(&mut self, span: Span) -> ArrayCtx {
        let start = self.idx.find_open_back_to_line(b'[', span.start());

        if let Some(pk) = self.inline_pending.take() {
            return ArrayCtx {
                path: pk.path,
                start,
                key_loc: Some(pk.key_loc),
                items: Vec::new(),
            };
        }
        if let Some(pk) = self.pending.take() {
            return ArrayCtx {
                path: pk.path,
                start,
                key_loc: Some(pk.key_loc),
                items: Vec::new(),
            };
        }
        if let Some(p) = self.current_array_item_path() {
            return ArrayCtx {
                path: p,
                start,
                key_loc: None,
                items: Vec::new(),
            };
        }
        ArrayCtx {
            path: Vec::new(),
            start,
            key_loc: None,
            items: Vec::new(),
        }
    }

    fn start_inline_ctx(&mut self, span: Span) -> InlineCtx {
        let start = self.idx.find_open_back_to_line(b'{', span.start());

        if let Some(pk) = self.inline_pending.take() {
            return InlineCtx {
                path: pk.path,
                start,
                key_loc: Some(pk.key_loc),
                children: rustc_hash::FxHashMap::default(),
            };
        }
        if let Some(pk) = self.pending.take() {
            return InlineCtx {
                path: pk.path,
                start,
                key_loc: Some(pk.key_loc),
                children: rustc_hash::FxHashMap::default(),
            };
        }
        if let Some(p) = self.current_array_item_path() {
            return InlineCtx {
                path: p,
                start,
                key_loc: None,
                children: rustc_hash::FxHashMap::default(),
            };
        }

        InlineCtx {
            path: Vec::new(),
            start,
            key_loc: None,
            children: rustc_hash::FxHashMap::default(),
        }
    }

    fn ensure_aot_array_at(&mut self, aot_path: &[PathSeg], key_loc: KeyLoc, value_loc: ValueLoc) {
        let node = MetaNode::Array {
            key: Some(key_loc),
            value: value_loc,
            items: Vec::new(),
        };
        self.insert_node_at(aot_path, node);
    }

    fn scalar_to_py_obj(
        &self,
        kind: ScalarKind,
        decoded: &Cow<'_, str>,
        raw_span: Range<usize>,
    ) -> PyResult<Py<PyAny>> {
        let py = self.py;
        match kind {
            ScalarKind::String => decoded.as_ref().into_py_any(py),
            ScalarKind::Boolean(v) => v.into_py_any(py),
            ScalarKind::Integer(radix) => {
                let bytes = decoded.as_bytes();
                let options = ParseIntegerOptions::new();
                if let Ok(i_64) = parse_int!(i64, bytes, &options, radix.value()) {
                    return i_64.into_py_any(py);
                }
                if let Some(big_int) = BigInt::parse_bytes(bytes, radix.value()) {
                    return big_int.into_py_any(py);
                }
                let mut err = TomlError::custom(
                    format!(
                        "invalid integer '{}'",
                        &self.doc[raw_span.start..raw_span.end.min(self.doc.len())]
                    ),
                    Some(raw_span.start..raw_span.end),
                );
                err.set_input(Some(self.doc));
                Err(TOMLDecodeError::new_err((
                    err.to_string(),
                    self.doc.to_string(),
                    raw_span.start,
                )))
            }
            ScalarKind::Float => {
                let bytes = decoded.as_bytes();
                let parsed: f64 = lexical_core::parse(bytes).map_err(|err| {
                    TOMLDecodeError::new_err((
                        format!("invalid float '{}': {err}", decoded.as_ref()),
                        self.doc.to_string(),
                        raw_span.start,
                    ))
                })?;
                parsed.into_py_any(py)
            }
            ScalarKind::DateTime => {
                let dt = decoded.parse::<Datetime>().map_err(|_| {
                    TOMLDecodeError::new_err((
                        format!("invalid datetime '{}'", decoded.as_ref()),
                        self.doc.to_string(),
                        raw_span.start,
                    ))
                })?;
                let bound_any: Bound<'py, PyAny> = match (dt.date, dt.time, dt.offset) {
                    (Some(date), Some(time), Some(offset)) => {
                        let tzinfo = Some(&create_timezone_from_offset(py, offset)?);
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
        }
    }
}

impl<'a, 'py> EventReceiver for Collector<'a, 'py> {
    fn array_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.parsing_table_header = true;
        self.header_is_array = false;
        self.header_keys.clear();
        self.header_key_locs.clear();
        self.pending = None;
        self.inline_pending = None;

        self.header_start = self.idx.find_open_back_to_line(b'[', span.start());
        self.header_end = self.idx.find_close_forward_to_line(b']', span.start());
    }

    fn array_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        self.parsing_table_header = false;
        self.pending = None;
        self.inline_pending = None;
    }

    fn inline_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) -> bool {
        let inline_ctx = self.start_inline_ctx(span);
        self.inline_stack.push(inline_ctx);
        true
    }

    fn inline_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        let Some(mut inline_ctx) = self.inline_stack.pop() else {
            return;
        };

        let start = inline_ctx.start;
        let mut end = span.end();
        if end <= start {
            end = self.idx.find_close_forward_to_line(b'}', span.start());
        }

        let raw = self.idx.slice(start, end).to_string();
        let value_loc = self.make_value_loc(start, end, raw);

        let node = MetaNode::Table {
            key: inline_ctx.key_loc.take(),
            value: value_loc,
            children: std::mem::take(&mut inline_ctx.children),
        };

        if let Some(parent_inline) = self.inline_stack.last_mut() {
            if let Some(PathSeg::Key(leaf)) = inline_ctx.path.last() {
                parent_inline.children.insert(leaf.clone(), node);
                return;
            }
        }

        if let Some(parent_arr) = self.array_stack.last_mut() {
            if matches!(inline_ctx.path.last(), Some(PathSeg::Index(_))) {
                parent_arr.items.push(node);
                return;
            }
        }

        self.insert_node_at(&inline_ctx.path, node);
    }

    fn array_open(&mut self, span: Span, _error: &mut dyn ErrorSink) -> bool {
        let array_ctx = self.start_array_ctx(span);
        self.array_stack.push(array_ctx);
        true
    }

    fn array_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        let Some(mut array_ctx) = self.array_stack.pop() else {
            return;
        };

        let start = array_ctx.start;
        let mut end = span.end();
        if end <= start {
            end = self.idx.find_close_forward_to_line(b']', span.start());
        }

        let raw = self.idx.slice(start, end).to_string();
        let value_loc = self.make_value_loc(start, end, raw);

        let node = MetaNode::Array {
            key: array_ctx.key_loc.take(),
            value: value_loc,
            items: std::mem::take(&mut array_ctx.items),
        };

        if let Some(parent) = self.array_stack.last_mut() {
            parent.items.push(node);
            return;
        }

        if let Some(parent_inline) = self.inline_stack.last_mut() {
            if let Some(PathSeg::Key(leaf)) = array_ctx.path.last() {
                parent_inline.children.insert(leaf.clone(), node);
                return;
            }
        }

        self.insert_node_at(&array_ctx.path, node);
    }

    fn simple_key(
        &mut self,
        span: Span,
        _encoding: Option<decoder::Encoding>,
        _error: &mut dyn ErrorSink,
    ) {
        let raw = self
            .source
            .get(Event::new_unchecked(EventKind::SimpleKey, None, span))
            .unwrap();

        let key_raw = raw.as_str().to_string();
        let key = key_raw.trim_matches('"').trim_matches('\'').to_string();

        if span.start() > 0 && self.idx.doc.as_bytes()[span.start() - 1] == b'[' {
            self.parsing_table_header = true;
            self.header_is_array = self.is_array_header_at(span.start());
            self.header_keys.clear();
            self.header_key_locs.clear();
            self.header_start = self.idx.find_open_back_to_line(b'[', span.start());
            self.header_end = self.idx.find_close_forward_to_line(b']', span.start());
        }

        let key_loc = self.make_key_loc(key.clone(), key_raw, span.start(), span.end());

        if self.parsing_table_header {
            self.header_keys.push(key.clone());
            self.header_key_locs.push(key_loc);

            self.pending = None;
            self.inline_pending = None;

            if span.end() < self.idx.doc.len() && self.idx.doc.as_bytes()[span.end()] == b']' {
                self.parsing_table_header = false;

                let hdr = Self::header_key_string(&self.header_keys);
                let mut p: Vec<PathSeg> =
                    self.header_keys.iter().cloned().map(PathSeg::Key).collect();

                if self.header_is_array {
                    let idx = *self.aot_counters.get(&hdr).unwrap_or(&0);
                    self.aot_counters.insert(hdr, idx + 1);
                    p.push(PathSeg::Index(idx));

                    let aot_path: Vec<PathSeg> =
                        self.header_keys.iter().cloned().map(PathSeg::Key).collect();

                    let top_key_loc =
                        self.header_key_locs
                            .first()
                            .cloned()
                            .unwrap_or_else(|| KeyLoc {
                                key: self.header_keys.first().cloned().unwrap_or_default(),
                                key_raw: self.header_keys.first().cloned().unwrap_or_default(),
                                key_line: 0,
                                key_col: (0, 0),
                            });

                    let raw_hdr = self
                        .idx
                        .slice(self.header_start, self.header_end)
                        .to_string();
                    let hdr_loc = self.make_value_loc(self.header_start, self.header_end, raw_hdr);

                    self.ensure_aot_array_at(
                        &aot_path,
                        top_key_loc,
                        ValueLoc {
                            value_raw: String::new(),
                            value_line: hdr_loc.value_line,
                            value_col: hdr_loc.value_col,
                        },
                    );

                    self.insert_node_at(
                        &p,
                        MetaNode::Table {
                            key: None,
                            value: hdr_loc,
                            children: rustc_hash::FxHashMap::default(),
                        },
                    );
                }

                self.current_table_path = p;
            }
            return;
        }

        if let Some(inline_ctx) = self.inline_stack.last() {
            let pk = Self::pending_for_inline_child(&inline_ctx.path, &key, key_loc);
            self.inline_pending = Some(pk);
            return;
        }

        let pk = self.pending_for_key(&key, key_loc);
        self.pending = Some(pk);
    }

    fn scalar(
        &mut self,
        span: Span,
        encoding: Option<decoder::Encoding>,
        error: &mut dyn ErrorSink,
    ) {
        let raw = self
            .source
            .get(Event::new_unchecked(EventKind::Scalar, encoding, span))
            .unwrap();

        let raw_str = raw.as_str().to_string();

        let mut decoded = Cow::Borrowed("");
        let kind = raw.decode_scalar(&mut decoded, error);

        let py_value = self
            .scalar_to_py_obj(kind, &decoded, span.start()..span.end())
            .unwrap_or_else(|_| self.py.None().into_py_any(self.py).unwrap());

        if let Some(pk) = self.inline_pending.take() {
            let value_loc = self.make_value_loc(span.start(), span.end(), raw_str);
            let node = MetaNode::Scalar {
                key: Some(pk.key_loc),
                value: value_loc,
                py_value,
            };

            if let Some(inline_ctx) = self.inline_stack.last_mut() {
                if let Some(PathSeg::Key(leaf)) = pk.path.last() {
                    inline_ctx.children.insert(leaf.clone(), node);
                    return;
                }
            }

            self.insert_node_at(&pk.path, node);
            return;
        }

        if let Some(pk) = self.pending.take() {
            let value_loc = self.make_value_loc(span.start(), span.end(), raw_str);
            let node = MetaNode::Scalar {
                key: Some(pk.key_loc),
                value: value_loc,
                py_value,
            };
            self.insert_node_at(&pk.path, node);
            return;
        }

        if self.array_stack.last().is_some() {
            let value_loc = self.make_value_loc(span.start(), span.end(), raw_str);
            if let Some(parent_arr) = self.array_stack.last_mut() {
                parent_arr.items.push(MetaNode::Scalar {
                    key: None,
                    value: value_loc,
                    py_value,
                });
            }
        }
    }
}

fn build_value_line<'py>(py: Python<'py>, (l1, l2): (usize, usize)) -> PyResult<Bound<'py, PyAny>> {
    if l1 == l2 {
        Ok(l1.into_bound_py_any(py)?)
    } else {
        Ok(PyTuple::new(py, [l1, l2])?.into_any())
    }
}

fn build_scalar_dict<'py>(
    py: Python<'py>,
    key: Option<&KeyLoc>,
    value: &ValueLoc,
    py_value: &Py<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py_dict = PyDict::new(py);

    if let Some(k) = key {
        py_dict.set_item("key", k.key.as_str())?;
        py_dict.set_item("key_raw", k.key_raw.as_str())?;
        py_dict.set_item("key_line", k.key_line)?;
        py_dict.set_item("key_col", PyTuple::new(py, [k.key_col.0, k.key_col.1])?)?;
    }

    py_dict.set_item("value_raw", value.value_raw.as_str())?;
    py_dict.set_item("value_line", build_value_line(py, value.value_line)?)?;
    py_dict.set_item(
        "value_col",
        PyTuple::new(py, [value.value_col.0, value.value_col.1])?,
    )?;
    py_dict.set_item("value", py_value.bind(py))?;

    Ok(py_dict.into_any())
}

fn build_container_dict<'py>(
    py: Python<'py>,
    key: &KeyLoc,
    value: &ValueLoc,
    py_value: Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py_dict = PyDict::new(py);
    py_dict.set_item("key", key.key.as_str())?;
    py_dict.set_item("key_raw", key.key_raw.as_str())?;
    py_dict.set_item("key_line", key.key_line)?;
    py_dict.set_item("key_col", PyTuple::new(py, [key.key_col.0, key.key_col.1])?)?;
    py_dict.set_item("value_raw", value.value_raw.as_str())?;
    py_dict.set_item("value_line", build_value_line(py, value.value_line)?)?;
    py_dict.set_item(
        "value_col",
        PyTuple::new(py, [value.value_col.0, value.value_col.1])?,
    )?;
    py_dict.set_item("value", py_value)?;
    Ok(py_dict.into_any())
}

fn build_container_no_key<'py>(
    py: Python<'py>,
    value: &ValueLoc,
    py_value: Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py_dict = PyDict::new(py);
    py_dict.set_item("value_raw", value.value_raw.as_str())?;
    py_dict.set_item("value_line", build_value_line(py, value.value_line)?)?;
    py_dict.set_item(
        "value_col",
        PyTuple::new(py, [value.value_col.0, value.value_col.1])?,
    )?;
    py_dict.set_item("value", py_value)?;
    Ok(py_dict.into_any())
}

fn build_tree_node<'py>(py: Python<'py>, node: &MetaNode) -> PyResult<Bound<'py, PyAny>> {
    match node {
        MetaNode::Root { children } => {
            let py_dict = PyDict::new(py);
            for (k, v) in children {
                py_dict.set_item(k.as_str(), build_tree_node(py, v)?)?;
            }
            Ok(py_dict.into_any())
        }

        MetaNode::Scalar {
            key,
            value,
            py_value,
        } => build_scalar_dict(py, key.as_ref(), value, py_value),

        MetaNode::Table {
            key,
            value,
            children,
        } => {
            let py_value_dict = PyDict::new(py);
            for (ck, cv) in children {
                py_value_dict.set_item(ck.as_str(), build_tree_node(py, cv)?)?;
            }

            match key.as_ref() {
                Some(k) => build_container_dict(py, k, value, py_value_dict.into_any()),
                None => {
                    if value.value_raw.is_empty() {
                        Ok(py_value_dict.into_any())
                    } else {
                        build_container_no_key(py, value, py_value_dict.into_any())
                    }
                }
            }
        }

        MetaNode::Array { key, value, items } => {
            let py_list = PyList::empty(py);
            for it in items {
                py_list.append(build_tree_node(py, it)?)?;
            }

            match key.as_ref() {
                Some(k) => build_container_dict(py, k, value, py_list.into_any()),
                None => {
                    if value.value_raw.is_empty() {
                        Ok(py_list.into_any())
                    } else {
                        build_container_no_key(py, value, py_list.into_any())
                    }
                }
            }
        }
    }
}

pub(crate) fn extract_metadata<'py>(
    py: Python<'py>,
    _table: &Spanned<DeTable<'_>>,
    doc: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let source = Source::new(doc);
    let tokens = source.lex().into_vec();

    let mut errors = Vec::new();
    let mut collector = Collector::new(py, &source, doc);

    parse_document(&tokens, &mut collector, &mut errors);

    let py_dict = PyDict::new(py);
    py_dict.set_item("tree", build_tree_node(py, &collector.root)?)?;
    Ok(py_dict.into_any())
}

pub(crate) fn to_python<'py>(
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
                let tzinfo = Some(&create_timezone_from_offset(py, offset)?);
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
                let key = k.get_ref().clone().into_owned();
                let val = to_python(py, v.get_ref(), v.span(), doc)?;
                py_dict.set_item(key, val)?;
            }
            Ok(py_dict.into_any())
        }
    }
}
