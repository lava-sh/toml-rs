use std::{borrow::Cow, ops::Range};

use lexical_core::ParseIntegerOptions;
use num_bigint::BigInt;
use pyo3::{
    Bound, IntoPyObjectExt, PyAny,
    prelude::*,
    types::{PyDate, PyDict, PyList, PyTime, PyTuple},
};
use toml_parser_v1::{
    ErrorSink, Source, Span, decoder,
    decoder::ScalarKind,
    parser::{Event, EventKind, EventReceiver, parse_document},
};
use toml_v1::{
    Spanned,
    de::{DeTable, DeValue},
    value::Datetime,
};

use crate::{
    core::{loads::create_timezone_from_offset, metadata::DocIndex},
    create_py_datetime_v1,
    error::TomlError,
    parse_int,
    toml_rs::TOMLDecodeError,
};

#[derive(Clone)]
struct KeyLoc {
    key: String,
    key_raw: String,
    key_line: usize,
    key_col: (usize, usize),
}

#[derive(Clone)]
struct ValueLoc {
    raw_span: Option<Range<usize>>,
    line: (usize, usize),
    col: (usize, usize),
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
    fn doc_slice(&self, span: Range<usize>) -> &str {
        self.doc.get(span).unwrap_or("")
    }

    fn inline_has_trailing_comma(&self, start: usize, end: usize) -> bool {
        let raw = self.doc_slice(start..end);
        let bytes = raw.as_bytes();
        if bytes.is_empty() {
            return false;
        }

        let mut i = bytes.len();
        while i > 0 {
            i -= 1;
            let b = bytes[i];
            if b.is_ascii_whitespace() {
                continue;
            }
            if b != b'}' {
                return false;
            }
            break;
        }

        while i > 0 {
            i -= 1;
            let b = bytes[i];
            if b.is_ascii_whitespace() {
                continue;
            }
            return b == b',';
        }

        false
    }

    fn empty_value_loc() -> ValueLoc {
        ValueLoc {
            raw_span: None,
            line: (0, 0),
            col: (0, 0),
        }
    }

    fn default_table_node() -> MetaNode {
        MetaNode::Table {
            key: None,
            value: Self::empty_value_loc(),
            children: rustc_hash::FxHashMap::default(),
        }
    }

    fn default_array_node() -> MetaNode {
        MetaNode::Array {
            key: None,
            value: Self::empty_value_loc(),
            items: Vec::new(),
        }
    }

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

    fn decode_key(key_raw: &str) -> String {
        let source = toml_parser::Source::new(key_raw);
        let mut errors = Vec::new();
        let keys = toml_edit::parse_key_path(source, &mut errors);
        if errors.is_empty()
            && let [key] = keys.as_slice()
        {
            return key.get().to_string();
        }

        if let Some(stripped) = key_raw.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            stripped.to_string()
        } else if let Some(stripped) = key_raw
            .strip_prefix('\'')
            .and_then(|s| s.strip_suffix('\''))
        {
            stripped.to_string()
        } else {
            key_raw.to_string()
        }
    }

    fn header_key_is_last_segment(&self, pos: usize) -> bool {
        let bytes = self.idx.doc.as_bytes();
        let end = self.header_end.min(bytes.len());
        let mut i = pos.min(end);

        while i < end {
            match bytes[i] {
                b']' => return true,
                b'.' => return false,
                _ => i += 1,
            }
        }
        false
    }

    fn make_key_loc(&self, key: &str, key_raw: &str, start: usize, end: usize) -> KeyLoc {
        let (line, _) = self.idx.line_col(start);
        let col = self.idx.col_range_same_line(start, end);
        KeyLoc {
            key: key.to_owned(),
            key_raw: key_raw.to_owned(),
            key_line: line,
            key_col: col,
        }
    }

    fn make_value_loc(&self, start: usize, end: usize) -> ValueLoc {
        let value_line = self.idx.value_line_range(start, end);
        let value_col = self.idx.value_col_range_first_line(start, end);
        ValueLoc {
            raw_span: Some(start..end),
            line: value_line,
            col: value_col,
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
                MetaNode::Root { children } | MetaNode::Table { children, .. } => children,
                _ => {
                    *cur = Collector::default_table_node();
                    match cur {
                        MetaNode::Table { children, .. } => children,
                        _ => unreachable!(),
                    }
                }
            }
        }

        fn ensure_array_items(cur: &mut MetaNode) -> &mut Vec<MetaNode> {
            if let MetaNode::Array { items, .. } = cur {
                items
            } else {
                *cur = Collector::default_array_node();
                match cur {
                    MetaNode::Array { items, .. } => items,
                    _ => unreachable!(),
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
                        .or_insert_with(Collector::default_table_node);
                }
                PathSeg::Index(idx) => {
                    let items = ensure_array_items(cur);
                    while items.len() <= *idx {
                        items.push(Collector::default_table_node());
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

    fn get_node_mut_at(&mut self, path: &[PathSeg]) -> Option<&mut MetaNode> {
        let mut cur = &mut self.root;
        for seg in path {
            match seg {
                PathSeg::Key(key) => {
                    cur = match cur {
                        MetaNode::Root { children } | MetaNode::Table { children, .. } => {
                            children.get_mut(key)?
                        }
                        _ => return None,
                    };
                }
                PathSeg::Index(idx) => {
                    cur = match cur {
                        MetaNode::Array { items, .. } => items.get_mut(*idx)?,
                        _ => return None,
                    };
                }
            }
        }
        Some(cur)
    }

    fn current_array_item_path(&self) -> Option<Vec<PathSeg>> {
        let ctx = self.array_stack.last()?;
        let idx = ctx.items.len();
        let mut p = ctx.path.clone();
        p.push(PathSeg::Index(idx));
        Some(p)
    }

    fn take_pending_key(&mut self) -> Option<PendingKey> {
        self.inline_pending.take().or_else(|| self.pending.take())
    }

    fn container_path_fallback(&self) -> Vec<PathSeg> {
        self.current_array_item_path().unwrap_or_default()
    }

    fn start_array_ctx(&mut self, span: Span) -> ArrayCtx {
        let start = self.idx.find_open_back_to_line(b'[', span.start());
        let (path, key_loc) = match self.take_pending_key() {
            Some(pk) => (pk.path, Some(pk.key_loc)),
            None => (self.container_path_fallback(), None),
        };
        ArrayCtx {
            path,
            start,
            key_loc,
            items: Vec::new(),
        }
    }

    fn start_inline_ctx(&mut self, span: Span) -> InlineCtx {
        let start = self.idx.find_open_back_to_line(b'{', span.start());
        let (path, key_loc) = match self.take_pending_key() {
            Some(pk) => (pk.path, Some(pk.key_loc)),
            None => (self.container_path_fallback(), None),
        };
        InlineCtx {
            path,
            start,
            key_loc,
            children: rustc_hash::FxHashMap::default(),
        }
    }

    fn ensure_aot_array_at(&mut self, aot_path: &[PathSeg], key_loc: KeyLoc, value_loc: ValueLoc) {
        if let Some(existing) = self.get_node_mut_at(aot_path) {
            if let MetaNode::Array { key, value, .. } = existing {
                if key.is_none() {
                    *key = Some(key_loc);
                }
                if value.raw_span.is_none() {
                    *value = value_loc;
                }
            }
            return;
        }

        self.insert_node_at(
            aot_path,
            MetaNode::Array {
                key: Some(key_loc),
                value: value_loc,
                items: Vec::new(),
            },
        );
    }

    fn scalar_to_py_obj(
        &self,
        kind: ScalarKind,
        decoded: &str,
        raw_span: Range<usize>,
    ) -> PyResult<Py<PyAny>> {
        let py = self.py;
        match kind {
            ScalarKind::String => decoded.into_py_any(py),
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
                        format!("invalid float '{decoded}': {err}"),
                        self.doc.to_string(),
                        raw_span.start,
                    ))
                })?;
                parsed.into_py_any(py)
            }
            ScalarKind::DateTime => {
                let dt = decoded.parse::<Datetime>().map_err(|_| {
                    TOMLDecodeError::new_err((
                        format!("invalid datetime '{decoded}'"),
                        self.doc.to_string(),
                        raw_span.start,
                    ))
                })?;
                let bound_any: Bound<'py, PyAny> = match (dt.date, dt.time, dt.offset) {
                    (Some(date), Some(time), Some(offset)) => {
                        let py_tzinfo = create_timezone_from_offset(py, offset)?;
                        let tzinfo = Some(&py_tzinfo);
                        create_py_datetime_v1!(py, date, time, tzinfo)?.into_any()
                    }
                    (Some(date), Some(time), None) => {
                        create_py_datetime_v1!(py, date, time, None)?.into_any()
                    }
                    (Some(date), None, None) => {
                        PyDate::new(py, i32::from(date.year), date.month, date.day)?.into_any()
                    }
                    (None, Some(time), None) => PyTime::new(
                        py,
                        time.hour,
                        time.minute,
                        time.second,
                        time.nanosecond / 1000,
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

impl EventReceiver for Collector<'_, '_> {
    fn array_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        self.parsing_table_header = true;
        self.header_is_array = true;
        self.header_keys.clear();
        self.header_key_locs.clear();
        self.pending = None;
        self.inline_pending = None;

        self.header_start = self.idx.find_open_back_to_line(b'[', span.start());
        if self.header_start > 0 && self.idx.doc.as_bytes()[self.header_start - 1] == b'[' {
            self.header_start -= 1;
        }
        self.header_end = self.idx.find_table_header_end(self.header_start, true);
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
        let end = span.end();
        if end <= start {
            return;
        }

        let line_range = self.idx.value_line_range(start, end);
        if line_range.0 != line_range.1 {
            return;
        }

        if self.inline_has_trailing_comma(start, end) {
            return;
        }

        let value_loc = self.make_value_loc(start, end);

        let node = MetaNode::Table {
            key: inline_ctx.key_loc.take(),
            value: value_loc,
            children: std::mem::take(&mut inline_ctx.children),
        };

        if let Some(parent_inline) = self.inline_stack.last_mut()
            && let Some(PathSeg::Key(leaf)) = inline_ctx.path.last()
        {
            parent_inline.children.insert(leaf.clone(), node);
            return;
        }

        if let Some(parent_arr) = self.array_stack.last_mut()
            && matches!(inline_ctx.path.last(), Some(PathSeg::Index(_)))
        {
            parent_arr.items.push(node);
            return;
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
        let end = span.end();
        if end <= start {
            return;
        }

        let value_loc = self.make_value_loc(start, end);

        let node = MetaNode::Array {
            key: array_ctx.key_loc.take(),
            value: value_loc,
            items: std::mem::take(&mut array_ctx.items),
        };

        if let Some(parent) = self.array_stack.last_mut() {
            parent.items.push(node);
            return;
        }

        if let Some(parent_inline) = self.inline_stack.last_mut()
            && let Some(PathSeg::Key(leaf)) = array_ctx.path.last()
        {
            parent_inline.children.insert(leaf.clone(), node);
            return;
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

        let key_raw = raw.as_str();
        let key = Self::decode_key(key_raw);

        if span.start() > 0 && self.idx.doc.as_bytes()[span.start() - 1] == b'[' {
            self.parsing_table_header = true;
            self.header_is_array = self.is_array_header_at(span.start());
            self.header_keys.clear();
            self.header_key_locs.clear();
            self.header_start = self.idx.find_open_back_to_line(b'[', span.start());
            if self.header_is_array
                && self.header_start > 0
                && self.idx.doc.as_bytes()[self.header_start - 1] == b'['
            {
                self.header_start -= 1;
            }
            self.header_end = self
                .idx
                .find_table_header_end(self.header_start, self.header_is_array);
        }

        let key_loc = self.make_key_loc(&key, key_raw, span.start(), span.end());

        if self.parsing_table_header {
            self.header_keys.push(key.clone());
            self.header_key_locs.push(key_loc);

            self.pending = None;
            self.inline_pending = None;

            if self.header_key_is_last_segment(span.end()) {
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

                    let hdr_loc = self.make_value_loc(self.header_start, self.header_end);

                    self.ensure_aot_array_at(
                        &aot_path,
                        top_key_loc,
                        ValueLoc {
                            raw_span: None,
                            line: hdr_loc.line,
                            col: hdr_loc.col,
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

        let mut decoded = Cow::Borrowed("");
        let kind = raw.decode_scalar(&mut decoded, error);

        let py_value = self
            .scalar_to_py_obj(kind, decoded.as_ref(), span.start()..span.end())
            .unwrap_or_else(|_| self.py.None().into_py_any(self.py).unwrap());

        if let Some(pk) = self.inline_pending.take() {
            let value_loc = self.make_value_loc(span.start(), span.end());
            let node = MetaNode::Scalar {
                key: Some(pk.key_loc),
                value: value_loc,
                py_value,
            };

            if let Some(inline_ctx) = self.inline_stack.last_mut()
                && let Some(PathSeg::Key(leaf)) = pk.path.last()
            {
                inline_ctx.children.insert(leaf.clone(), node);
                return;
            }

            self.insert_node_at(&pk.path, node);
            return;
        }

        if let Some(pk) = self.pending.take() {
            let value_loc = self.make_value_loc(span.start(), span.end());
            let node = MetaNode::Scalar {
                key: Some(pk.key_loc),
                value: value_loc,
                py_value,
            };
            self.insert_node_at(&pk.path, node);
            return;
        }

        if self.array_stack.last().is_some() {
            let value_loc = self.make_value_loc(span.start(), span.end());
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

fn build_value_line(py: Python, (l1, l2): (usize, usize)) -> PyResult<Bound<PyAny>> {
    if l1 == l2 {
        Ok(l1.into_bound_py_any(py)?)
    } else {
        Ok(PyTuple::new(py, [l1, l2])?.into_any())
    }
}

fn build_key_col(py: Python, (c1, c2): (usize, usize)) -> PyResult<Bound<PyAny>> {
    if c1 == c2 {
        Ok(c1.into_bound_py_any(py)?)
    } else {
        Ok(PyTuple::new(py, [c1, c2])?.into_any())
    }
}

fn build_value_col(py: Python, (c1, c2): (usize, usize)) -> PyResult<Bound<PyAny>> {
    if c1 == c2 {
        Ok(c1.into_bound_py_any(py)?)
    } else {
        Ok(PyTuple::new(py, [c1, c2])?.into_any())
    }
}

fn value_raw<'a>(doc: &'a str, value: &ValueLoc) -> Cow<'a, str> {
    match &value.raw_span {
        Some(span) if span.start < span.end && span.end <= doc.len() => {
            Cow::Borrowed(&doc[span.start..span.end])
        }
        _ => Cow::Borrowed(""),
    }
}

fn build_scalar_dict<'py>(
    py: Python<'py>,
    doc: &str,
    key: Option<&KeyLoc>,
    value: &ValueLoc,
    py_value: &Py<PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py_dict = PyDict::new(py);

    if let Some(k) = key {
        py_dict.set_item("key", k.key.as_str())?;
        py_dict.set_item("key_raw", k.key_raw.as_str())?;
        py_dict.set_item("key_line", k.key_line)?;
        py_dict.set_item("key_col", build_key_col(py, k.key_col)?)?;
    }

    let value_raw = value_raw(doc, value);
    py_dict.set_item("value_raw", value_raw.as_ref())?;
    py_dict.set_item("value_line", build_value_line(py, value.line)?)?;
    py_dict.set_item("value_col", build_value_col(py, value.col)?)?;
    py_dict.set_item("value", py_value.bind(py))?;

    Ok(py_dict.into_any())
}

fn set_value_fields<'py>(
    py: Python<'py>,
    doc: &str,
    py_dict: &Bound<'py, PyDict>,
    value: &ValueLoc,
    py_value: Bound<'py, PyAny>,
) -> PyResult<()> {
    py_dict.set_item("value", py_value)?;
    if value.raw_span.is_some() {
        let value_raw = value_raw(doc, value);
        py_dict.set_item("value_raw", value_raw.as_ref())?;
        py_dict.set_item("value_line", build_value_line(py, value.line)?)?;
        py_dict.set_item("value_col", build_value_col(py, value.col)?)?;
    }
    Ok(())
}

fn build_dict<'py>(
    py: Python<'py>,
    doc: &str,
    key: Option<&KeyLoc>,
    value: &ValueLoc,
    py_value: Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py_dict = PyDict::new(py);

    if let Some(k) = key {
        py_dict.set_item("key", k.key.as_str())?;
        py_dict.set_item("key_raw", k.key_raw.as_str())?;
        py_dict.set_item("key_line", k.key_line)?;
        py_dict.set_item("key_col", build_key_col(py, k.key_col)?)?;
    }

    set_value_fields(py, doc, &py_dict, value, py_value)?;
    Ok(py_dict.into_any())
}

fn build_nodes<'py>(py: Python<'py>, doc: &str, node: &MetaNode) -> PyResult<Bound<'py, PyAny>> {
    match node {
        MetaNode::Root { children } => {
            let py_dict = PyDict::new(py);
            for (k, v) in children {
                py_dict.set_item(k.as_str(), build_nodes(py, doc, v)?)?;
            }
            Ok(py_dict.into_any())
        }

        MetaNode::Scalar {
            key,
            value,
            py_value,
        } => build_scalar_dict(py, doc, key.as_ref(), value, py_value),

        MetaNode::Table {
            key,
            value,
            children,
        } => {
            let py_value_dict = PyDict::new(py);
            for (ck, cv) in children {
                py_value_dict.set_item(ck.as_str(), build_nodes(py, doc, cv)?)?;
            }

            match key.as_ref() {
                Some(k) => build_dict(py, doc, Some(k), value, py_value_dict.into_any()),
                None => {
                    if value.raw_span.is_none() {
                        Ok(py_value_dict.into_any())
                    } else {
                        build_dict(py, doc, None, value, py_value_dict.into_any())
                    }
                }
            }
        }

        MetaNode::Array { key, value, items } => {
            let py_list = PyList::empty(py);
            for it in items {
                py_list.append(build_nodes(py, doc, it)?)?;
            }

            match key.as_ref() {
                Some(k) => build_dict(py, doc, Some(k), value, py_list.into_any()),
                None => {
                    if value.raw_span.is_none() {
                        Ok(py_list.into_any())
                    } else {
                        build_dict(py, doc, None, value, py_list.into_any())
                    }
                }
            }
        }
    }
}

pub(crate) fn extract_metadata_v1<'py>(
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
    py_dict.set_item("nodes", build_nodes(py, doc, &collector.root)?)?;
    Ok(py_dict.into_any())
}

pub(crate) fn to_python_v1<'py>(
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
                Ok(create_py_datetime_v1!(py, date, time, tzinfo)?.into_any())
            }
            (Some(date), Some(time), None) => {
                Ok(create_py_datetime_v1!(py, date, time, None)?.into_any())
            }
            (Some(date), None, None) => {
                Ok(PyDate::new(py, i32::from(date.year), date.month, date.day)?.into_any())
            }
            (None, Some(time), None) => Ok(PyTime::new(
                py,
                time.hour,
                time.minute,
                time.second,
                time.nanosecond / 1000,
                None,
            )?
            .into_any()),
            _ => unreachable!(),
        },
        DeValue::Array(array) => {
            let py_list = PyList::empty(py);
            for item in array {
                py_list.append(to_python_v1(py, item.get_ref(), item.span(), doc)?)?;
            }
            Ok(py_list.into_any())
        }
        DeValue::Table(table) => {
            let py_dict = PyDict::new(py);
            for (k, v) in table {
                let val = to_python_v1(py, v.get_ref(), v.span(), doc)?;
                py_dict.set_item(k.get_ref().as_ref(), val)?;
            }
            Ok(py_dict.into_any())
        }
    }
}
