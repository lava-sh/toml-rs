use std::{borrow::Cow, ops::Range};

use lexical_core::ParseIntegerOptions;
use memchr::{memchr, memchr_iter, memrchr};
use num_bigint::BigInt;
use pyo3::{
    Bound, IntoPyObjectExt, PyAny, PyResult, Python,
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
    de::{DeArray, DeFloat, DeInteger, DeTable, DeValue},
    value::Datetime,
};

use crate::{
    create_py_datetime_v1, error::TomlError, parse_int, toml_rs::TOMLDecodeError,
    v1::loads::create_timezone_from_offset,
};

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
}

#[derive(Debug, Clone)]
struct KeyMeta<'a> {
    // key
    key_line: usize,
    key_col: usize,
    key_span: (usize, usize),
    value: DeValue<'a>,
    // value
    value_raw: String,
    value_line_start: usize,
    value_line_end: usize,
    value_col: usize,
    value_span: (usize, usize),
}

#[derive(Debug, Clone)]
struct PendingKey {
    full_key: String,
    leaf_key: String,
    key_line: usize,
    key_col: usize,
    key_span: (usize, usize),
}

#[derive(Debug)]
struct Inline<'a> {
    full_key: String,
    start: usize,
    entries: Vec<(String, Spanned<DeValue<'a>>)>,
}

struct Collector<'a> {
    source: &'a Source<'a>,
    idx: DocIndex<'a>,
    keys: rustc_hash::FxHashMap<String, KeyMeta<'a>>,
    current_path: Vec<String>,
    parsing_table_header: bool,
    pending: Option<PendingKey>,
    inline_pending: Option<PendingKey>,
    inline_stack: Vec<Inline<'a>>,
    array_stack: Vec<Vec<Spanned<DeValue<'a>>>>,
    array_start_stack: Vec<usize>,
}

impl<'a> Collector<'a> {
    fn new(source: &'a Source<'a>, doc: &'a str) -> Self {
        Self {
            source,
            idx: DocIndex::new(doc),
            keys: rustc_hash::FxHashMap::default(),
            current_path: Vec::new(),
            parsing_table_header: false,
            pending: None,
            inline_pending: None,
            inline_stack: Vec::new(),
            array_stack: Vec::new(),
            array_start_stack: Vec::new(),
        }
    }

    fn full_key(&self, key: &str) -> String {
        if self.current_path.is_empty() {
            key.to_string()
        } else {
            format!("{}.{}", self.current_path.join("."), key)
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_value(
        &mut self,
        full_key: String,
        key_line: usize,
        key_col: usize,
        key_span: (usize, usize),
        value: DeValue<'a>,
        start: usize,
        end: usize,
        raw: String,
    ) {
        let (line_start, col) = self.idx.line_col(start);

        let end_pos = end
            .saturating_sub(1)
            .min(self.idx.doc.len().saturating_sub(1));
        let (line_end, _) = self.idx.line_col(end_pos);

        let meta = KeyMeta {
            // key
            key_line,
            key_col,
            key_span,
            value,
            // value
            value_raw: raw,
            value_line_start: line_start,
            value_line_end: line_end,
            value_col: col,
            value_span: (start, end),
        };

        self.keys.insert(full_key, meta);
    }

    fn attach_value_to_inline(
        &mut self,
        leaf_key: &str,
        start: usize,
        end: usize,
        value: DeValue<'a>,
    ) {
        if let Some(ctx) = self.inline_stack.last_mut() {
            ctx.entries
                .push((leaf_key.to_string(), Spanned::new(start..end, value)));
        }
    }

    fn push_inline_ctx_from_pending(&mut self, start: usize) -> Option<Inline<'a>> {
        if let Some(pk) = self.inline_pending.take() {
            let parent = self.inline_stack.last().map(|c| c.full_key.as_str());
            let full = if let Some(p) = parent {
                format!("{}.{}", p, pk.leaf_key)
            } else {
                pk.full_key.clone()
            };
            self.inline_pending = Some(pk);
            return Some(Inline {
                full_key: full,
                start,
                entries: Vec::new(),
            });
        }
        if let Some(pk) = self.pending.take() {
            return Some(Inline {
                full_key: pk.full_key.clone(),
                start,
                entries: Vec::new(),
            });
        }
        None
    }
}

impl EventReceiver for Collector<'_> {
    fn array_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        self.parsing_table_header = true;
        self.current_path.clear();
        self.pending = None;
        self.inline_pending = None;
    }

    fn array_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        self.parsing_table_header = false;
        self.pending = None;
        self.inline_pending = None;
    }

    fn inline_table_open(&mut self, span: Span, _error: &mut dyn ErrorSink) -> bool {
        let start = self.idx.find_open_back_to_line(b'{', span.start());
        if let Some(ctx) = self.push_inline_ctx_from_pending(start) {
            self.inline_stack.push(ctx);
        } else {
            self.inline_stack.push(Inline {
                full_key: String::new(),
                start,
                entries: Vec::new(),
            });
        }
        true
    }

    fn inline_table_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        let Some(mut ctx) = self.inline_stack.pop() else {
            return;
        };

        let start = ctx.start;
        let mut end = span.end();
        if end <= start {
            end = self.idx.find_close_forward_to_line(b'}', span.start());
        }

        let mut table = DeTable::new();
        for (k, v) in ctx.entries.drain(..) {
            table.insert(Spanned::new(0..0, Cow::Owned(k)), v);
        }
        let value = DeValue::Table(table);

        if let Some(pk) = self.inline_pending.take() {
            let PendingKey {
                full_key,
                leaf_key,
                key_line,
                key_col,
                key_span,
            } = pk;

            let raw = self.idx.slice(start, end).to_string();

            self.emit_value(
                full_key,
                key_line,
                key_col,
                key_span,
                value.clone(),
                start,
                end,
                raw,
            );

            self.attach_value_to_inline(&leaf_key, start, end, value);
            return;
        }

        if ctx.full_key.is_empty() {
            return;
        }

        let raw = self.idx.slice(start, end).to_string();
        let (line_start, col) = self.idx.line_col(start);
        let end_pos = end
            .saturating_sub(1)
            .min(self.idx.doc.len().saturating_sub(1));
        let (line_end, _) = self.idx.line_col(end_pos);

        self.keys.insert(
            ctx.full_key,
            KeyMeta {
                // key
                key_line: 0,
                key_col: 0,
                key_span: (start, end),
                value,
                // value
                value_raw: raw,
                value_line_start: line_start,
                value_line_end: line_end,
                value_col: col,
                value_span: (start, end),
            },
        );
    }

    fn array_open(&mut self, span: Span, _error: &mut dyn ErrorSink) -> bool {
        let start = self.idx.find_open_back_to_line(b'[', span.start());
        self.array_start_stack.push(start);
        self.array_stack.push(Vec::new());
        true
    }

    fn array_close(&mut self, span: Span, _error: &mut dyn ErrorSink) {
        let items = self.array_stack.pop().unwrap_or_default();
        let start = self.array_start_stack.pop().unwrap_or(span.start());

        let mut end = span.end();
        if end <= start {
            end = self.idx.find_close_forward_to_line(b']', span.start());
        }

        let arr = build_dearray(items);
        let value = DeValue::Array(arr);

        if !self.array_stack.is_empty() {
            self.array_stack
                .last_mut()
                .unwrap()
                .push(Spanned::new(start..end, value));
            return;
        }

        if let Some(pk) = self.inline_pending.take() {
            let PendingKey {
                full_key,
                leaf_key,
                key_line,
                key_col,
                key_span,
            } = pk;

            let raw = self.idx.slice(start, end).to_string();

            self.emit_value(
                full_key,
                key_line,
                key_col,
                key_span,
                value.clone(),
                start,
                end,
                raw,
            );

            self.attach_value_to_inline(&leaf_key, start, end, value);
            return;
        }

        if let Some(pk) = self.pending.take() {
            let PendingKey {
                full_key,
                key_line,
                key_col,
                key_span,
                ..
            } = pk;

            let raw = self.idx.slice(start, end).to_string();

            self.emit_value(
                full_key, key_line, key_col, key_span, value, start, end, raw,
            );
        }
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

        let key_str = raw.as_str().to_string();
        let (line, col) = self.idx.line_col(span.start());

        if span.start() > 0 && self.idx.doc.as_bytes()[span.start() - 1] == b'[' {
            self.parsing_table_header = true;
            self.current_path.clear();
        }

        if self.parsing_table_header {
            self.current_path.push(key_str);
            self.pending = None;
            self.inline_pending = None;
            if span.end() < self.idx.doc.len() && self.idx.doc.as_bytes()[span.end()] == b']' {
                self.parsing_table_header = false;
            }
            return;
        }

        if !self.inline_stack.is_empty() {
            let parent = self.inline_stack.last().map_or("", |c| c.full_key.as_str());
            let full_key = if parent.is_empty() {
                key_str.clone()
            } else {
                format!("{parent}.{key_str}")
            };
            self.inline_pending = Some(PendingKey {
                full_key,
                leaf_key: key_str,
                key_line: line,
                key_col: col,
                key_span: (span.start(), span.end()),
            });
            return;
        }

        let full_key = self.full_key(&key_str);
        self.pending = Some(PendingKey {
            full_key,
            leaf_key: key_str,
            key_line: line,
            key_col: col,
            key_span: (span.start(), span.end()),
        });
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

        let raw_str = raw.as_str();
        let mut decoded = Cow::Borrowed("");
        let kind = raw.decode_scalar(&mut decoded, error);

        let value = match kind {
            ScalarKind::String => DeValue::String(decoded),
            ScalarKind::Boolean(v) => DeValue::Boolean(v),
            ScalarKind::DateTime => decoded.parse::<Datetime>().map_or_else(
                |_| {
                    DeValue::Datetime(Datetime {
                        date: None,
                        time: None,
                        offset: None,
                    })
                },
                DeValue::Datetime,
            ),
            ScalarKind::Float => DeValue::Float(DeFloat { inner: decoded }),
            ScalarKind::Integer(radix) => DeValue::Integer(DeInteger {
                inner: decoded,
                radix: radix.value(),
            }),
        };

        if !self.array_stack.is_empty() {
            self.array_stack
                .last_mut()
                .unwrap()
                .push(Spanned::new(span.start()..span.end(), value));
            return;
        }

        if let Some(pk) = self.inline_pending.take() {
            let PendingKey {
                full_key,
                leaf_key,
                key_line,
                key_col,
                key_span,
            } = pk;

            let raw = raw_str.to_string();

            self.emit_value(
                full_key,
                key_line,
                key_col,
                key_span,
                value.clone(),
                span.start(),
                span.end(),
                raw,
            );

            self.attach_value_to_inline(&leaf_key, span.start(), span.end(), value);
            return;
        }

        if let Some(pk) = self.pending.take() {
            let PendingKey {
                full_key,
                key_line,
                key_col,
                key_span,
                ..
            } = pk;

            self.emit_value(
                full_key,
                key_line,
                key_col,
                key_span,
                value,
                span.start(),
                span.end(),
                raw_str.to_string(),
            );
        }
    }
}

fn build_dearray(items: Vec<Spanned<DeValue>>) -> DeArray {
    let mut array = DeArray::new();
    for item in items {
        array.push(item);
    }
    array
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
                let tzinfo = Some(&create_timezone_from_offset(py, offset)?);
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
                let key = k.get_ref().clone().into_owned();
                let val = to_python_v1(py, v.get_ref(), v.span(), doc)?;
                py_dict.set_item(key, val)?;
            }
            Ok(py_dict.into_any())
        }
    }
}

fn build_py_dict<'py>(
    py: Python<'py>,
    map: &rustc_hash::FxHashMap<String, KeyMeta<'_>>,
    doc: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let py_dict = PyDict::new(py);
    for (key, meta) in map {
        let d = PyDict::new(py);
        d.set_item("key", key)?;
        d.set_item("key_line", meta.key_line)?;
        d.set_item("key_col", meta.key_col)?;
        d.set_item(
            "key_span",
            PyTuple::new(py, [meta.key_span.0, meta.key_span.1])?,
        )?;
        d.set_item(
            "value",
            to_python_v1(py, &meta.value, meta.value_span.0..meta.value_span.1, doc)?,
        )?;
        d.set_item("value_raw", meta.value_raw.as_str())?;
        if meta.value_line_start == meta.value_line_end {
            d.set_item("value_line", meta.value_line_start)?;
        } else {
            d.set_item(
                "value_line",
                PyTuple::new(py, [meta.value_line_start, meta.value_line_end])?,
            )?;
        }
        d.set_item("value_col", meta.value_col)?;
        d.set_item(
            "value_span",
            PyTuple::new(py, [meta.value_span.0, meta.value_span.1])?,
        )?;
        py_dict.set_item(key, d)?;
    }
    Ok(py_dict)
}

pub(crate) fn extract_metadata_v1<'py>(
    py: Python<'py>,
    _table: &Spanned<DeTable<'_>>,
    doc: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let source = Source::new(doc);
    let tokens = source.lex().into_vec();

    let mut errors = Vec::new();
    let mut collector = Collector::new(&source, doc);

    parse_document(&tokens, &mut collector, &mut errors);

    let dict = PyDict::new(py);
    dict.set_item("keys", build_py_dict(py, &collector.keys, doc)?)?;
    Ok(dict.into_any())
}
