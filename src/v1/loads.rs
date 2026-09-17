use std::{borrow::Cow, fmt::Write as _};

use num_bigint::BigInt;
use pyo3::{
    IntoPyObjectExt,
    exceptions::PyValueError,
    ffi,
    prelude::*,
    types::{PyBool, PyDate, PyDelta, PyDict, PyFloat, PyInt, PyList, PyString, PyTime, PyTzInfo},
};
use rustc_hash::FxHashMap;
use toml_parser_v1::{
    ErrorSink, Expected, ParseError, Raw, Source, Span,
    decoder::{Encoding, ScalarKind},
    parser::{EventReceiver, ValidateWhitespace},
};
use toml_v1::value::Offset;

use crate::{create_py_datetime_v1, error::DecodeError, parse_int};

trait PyBuild<'py>: Copy {
    fn py(self) -> Python<'py>;

    #[inline]
    fn dict(self) -> Bound<'py, PyDict> {
        PyDict::new(self.py())
    }

    #[inline]
    fn list(self) -> Bound<'py, PyList> {
        PyList::empty(self.py())
    }

    #[inline]
    fn key(self, text: &str) -> Bound<'py, PyString> {
        PyString::new(self.py(), text)
    }

    #[inline]
    fn flag(self, value: bool) -> Bound<'py, PyAny> {
        PyBool::new(self.py(), value).to_owned().into_any()
    }

    fn int(self, value: i64) -> PyResult<Bound<'py, PyAny>> {
        let py = self.py();

        // SAFETY: `PyLong_FromLongLong` returns a new reference or NULL.
        let int = unsafe { Bound::from_owned_ptr_or_err(py, ffi::PyLong_FromLongLong(value)) }?;

        Ok(int.into_any())
    }

    #[inline]
    fn float(self, value: f64) -> PyResult<Bound<'py, PyAny>> {
        let py = self.py();

        // SAFETY: `PyFloat_FromDouble` returns a new reference or NULL.
        unsafe { Bound::from_owned_ptr_or_err(py, ffi::PyFloat_FromDouble(value)) }
    }

    #[inline]
    fn string(self, text: &str) -> Bound<'py, PyAny> {
        self.key(text).into_any()
    }

    #[inline]
    fn put(
        self,
        table: &Bound<'py, PyDict>,
        name: &Bound<'py, PyString>,
        value: &Bound<'py, PyAny>,
    ) -> PyResult<bool> {
        let py = self.py();

        // SAFETY: `PyDict_Size` cannot fail, and `PyDict_SetItem` increfs the
        // key and the value itself.
        let before = unsafe { ffi::PyDict_Size(table.as_ptr()) };

        // SAFETY: as above
        if unsafe { ffi::PyDict_SetItem(table.as_ptr(), name.as_ptr(), value.as_ptr()) } != 0 {
            return Err(PyErr::fetch(py));
        }

        // SAFETY: as above
        Ok(unsafe { ffi::PyDict_Size(table.as_ptr()) } != before)
    }

    #[inline]
    fn push(self, list: &Bound<'py, PyList>, value: &Bound<'py, PyAny>) -> PyResult<()> {
        // SAFETY: `PyList_Append` increfs `value` itself.
        if unsafe { ffi::PyList_Append(list.as_ptr(), value.as_ptr()) } == 0 {
            Ok(())
        } else {
            Err(PyErr::fetch(self.py()))
        }
    }

    #[inline]
    fn get(
        self,
        table: &Bound<'py, PyDict>,
        name: &Bound<'py, PyString>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let py = self.py();

        // SAFETY: `PyDict_GetItemWithError` returns a borrowed reference and
        // reports "missing" separately from errors.
        let item = unsafe { ffi::PyDict_GetItemWithError(table.as_ptr(), name.as_ptr()) };

        if item.is_null() {
            if PyErr::occurred(py) {
                return Err(PyErr::fetch(py));
            }

            return Ok(None);
        }

        // SAFETY: `item` is a borrowed reference to an object the table owns,
        // and the table is alive for as long as the borrow is.
        Ok(Some(unsafe { Bound::from_borrowed_ptr(py, item) }))
    }
}

impl<'py> PyBuild<'py> for Python<'py> {
    #[inline]
    fn py(self) -> Self {
        self
    }
}

fn type_str(value: &Bound<'_, PyAny>) -> &'static str {
    if value.is_instance_of::<PyBool>() {
        "boolean"
    } else if value.is_exact_instance_of::<PyInt>() {
        "integer"
    } else if value.is_exact_instance_of::<PyFloat>() {
        "float"
    } else if value.is_instance_of::<PyString>() {
        "string"
    } else if value.is_exact_instance_of::<PyList>() {
        "array"
    } else if value.is_exact_instance_of::<PyDict>() {
        "table"
    } else if value.is_instance_of::<PyDate>() || value.is_instance_of::<PyTime>() {
        "datetime"
    } else {
        "value"
    }
}

#[cold]
fn push_expected(message: &mut String, expected: &Expected) {
    match expected {
        Expected::Literal(literal) => match *literal {
            "\n" => message.push_str("newline"),
            "`" => message.push_str("'`'"),
            literal if literal.chars().all(|c| c.is_ascii_control()) => {
                let _ = write!(message, "`{}`", literal.escape_debug());
            }
            literal => {
                message.push('`');
                message.push_str(literal);
                message.push('`');
            }
        },
        Expected::Description(description) => message.push_str(description),
        _ => message.push_str("etc"),
    }
}

#[cold]
fn to_py_error(py: Python<'_>, input: &str, error: &ParseError) -> PyErr {
    let mut message = String::from(error.description());

    if let Some(expected) = error.expected() {
        message.push_str(", expected ");

        if expected.is_empty() {
            message.push_str("nothing");
        } else {
            for (index, expected) in expected.iter().enumerate() {
                if index != 0 {
                    message.push_str(", ");
                }

                push_expected(&mut message, expected);
            }
        }
    }

    let span = error.unexpected().map(|span| span.start()..span.end());

    span.map_or_else(
        || DecodeError::raw(format!("{message}\n"), input, 0).raised(py),
        |span| DecodeError::snippet(&message, input, span).raised(py),
    )
}

#[inline]
pub fn create_timezone_from_offset(py: Python, offset: Offset) -> PyResult<Bound<PyTzInfo>> {
    const SECS_IN_DAY: i32 = 86_400;

    match offset {
        Offset::Z => PyTzInfo::utc(py).map(Borrowed::to_owned),
        Offset::Custom { minutes } => {
            let seconds = i32::from(minutes) * 60;
            let days = seconds.div_euclid(SECS_IN_DAY);
            let seconds = seconds.rem_euclid(SECS_IN_DAY);
            let py_delta = PyDelta::new(py, days, seconds, 0, false)?;
            PyTzInfo::fixed_offset(py, py_delta)
        }
    }
}

struct Key<'i> {
    span: Span,
    text: Cow<'i, str>,
}

#[derive(Clone, Copy, Default)]
struct Flags {
    implicit: bool,
    dotted: bool,
    inline: bool,
}

#[derive(Clone, Copy)]
enum Container {
    Table(Flags),
    Array { aot: bool },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Scope {
    Document,
    Inline,
}

enum Slot<'py> {
    Document,
    Inline(Bound<'py, PyDict>),
}

impl Slot<'_> {
    fn scope(&self) -> Scope {
        match self {
            Self::Document => Scope::Document,
            Self::Inline(_) => Scope::Inline,
        }
    }
}

enum Target<'py, 'i> {
    Slot {
        slot: Slot<'py>,
        path: Vec<Key<'i>>,
        key: Key<'i>,
    },
    Array(Bound<'py, PyList>),
    Inline(Bound<'py, PyDict>),
}

/// Skip an event once the document has already failed.
macro_rules! guard {
    ($this:ident) => {
        if $this.bail {
            return;
        }
    };
    ($this:ident => $default:expr) => {
        if $this.bail {
            return $default;
        }
    };
}

struct RawReceiver<'py, 'i, 'a> {
    py: Python<'py>,
    source: Source<'i>,
    parse_float: &'a Bound<'py, PyAny>,
    root: Bound<'py, PyDict>,
    table: Bound<'py, PyDict>,
    targets: Vec<Target<'py, 'i>>,
    keys: Vec<Key<'i>>,
    containers: FxHashMap<usize, Container>,
    header: Option<bool>,
    semantic: Option<ParseError>,
    deferred: Option<ParseError>,
    value_error: Option<PyErr>,
    bail: bool,
}

impl<'py, 'i> RawReceiver<'py, 'i, '_> {
    #[inline]
    fn finish(&mut self, result: PyResult<bool>) {
        match result {
            Ok(true) => {}
            Ok(false) => self.bail = true,
            Err(error) => self.failed(error),
        }
    }

    #[inline]
    fn container(&self, object: &Bound<'py, PyAny>) -> Option<Container> {
        self.containers.get(&(object.as_ptr() as usize)).copied()
    }

    #[inline]
    fn note(&mut self, object: &Bound<'py, PyAny>, container: Container) {
        self.containers.insert(object.as_ptr() as usize, container);
    }

    #[inline]
    fn flags(&self, object: &Bound<'py, PyAny>) -> Flags {
        match self.container(object) {
            Some(Container::Table(flags)) => flags,
            _ => Flags::default(),
        }
    }

    #[inline]
    fn new_table(&mut self, flags: Flags) -> Bound<'py, PyDict> {
        let table = self.py.dict();

        self.note(&table, Container::Table(flags));

        table
    }

    #[inline]
    fn new_list(&mut self, aot: bool) -> Bound<'py, PyList> {
        let list = self.py.list();

        self.note(&list, Container::Array { aot });

        list
    }

    #[cold]
    fn report(&mut self, message: impl Into<Cow<'static, str>>, span: Span) {
        self.bail = true;
        self.semantic
            .get_or_insert_with(|| ParseError::new(message).with_unexpected(span));
    }

    #[cold]
    fn report_error(&mut self, error: ParseError) {
        self.bail = true;
        self.semantic.get_or_insert(error);
    }

    #[cold]
    fn defer(&mut self, message: impl Into<Cow<'static, str>>, span: Span) {
        self.deferred
            .get_or_insert_with(|| ParseError::new(message).with_unexpected(span));
    }

    #[inline]
    fn violate(&mut self, defer: bool, message: impl Into<Cow<'static, str>>, span: Span) {
        if defer {
            self.defer(message, span);
        } else {
            self.report(message, span);
        }
    }

    #[inline]
    fn flush_deferred(&mut self) {
        if let Some(error) = self.deferred.take() {
            self.report_error(error);
        }
    }

    #[cold]
    fn failed(&mut self, error: PyErr) {
        self.bail = true;
        self.value_error.get_or_insert(error);
    }

    #[inline]
    fn raw_at(&self, span: Span, encoding: Option<Encoding>) -> Raw<'i> {
        let input = self.source.input();

        // SAFETY: `span` comes from the lexer over this exact input.
        unsafe {
            Raw::new_unchecked(
                input.get_unchecked(span.start()..span.end()),
                encoding,
                span,
            )
        }
    }

    fn decode_key(
        &self,
        span: Span,
        encoding: Option<Encoding>,
    ) -> Result<Cow<'i, str>, ParseError> {
        let mut decoded: Cow<'i, str> = Cow::Borrowed("");
        let mut errors = Option::<ParseError>::None;

        self.raw_at(span, encoding)
            .decode_key(&mut decoded, &mut errors);

        errors.map_or(Ok(decoded), Err)
    }

    fn decode_scalar(
        &self,
        span: Span,
        encoding: Option<Encoding>,
    ) -> Result<(ScalarKind, Cow<'i, str>), ParseError> {
        let mut decoded: Cow<'i, str> = Cow::Borrowed("");
        let mut errors = Option::<ParseError>::None;
        let kind = self
            .raw_at(span, encoding)
            .decode_scalar(&mut decoded, &mut errors);

        errors.map_or(Ok((kind, decoded)), Err)
    }

    fn descend(
        &mut self,
        from: Bound<'py, PyDict>,
        path: &[Key<'i>],
        dotted: bool,
        scope: Scope,
        defer: bool,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        let mut current = from;

        for segment in path {
            let Some(child) = self.step(&current, segment, dotted, scope, defer)? else {
                return Ok(None);
            };

            current = child;
        }

        Ok(Some(current))
    }

    fn step(
        &mut self,
        from: &Bound<'py, PyDict>,
        segment: &Key<'i>,
        dotted: bool,
        scope: Scope,
        defer: bool,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        let py = self.py;
        let name = py.key(&segment.text);

        let Some(existing) = py.get(from, &name)? else {
            let table = self.new_table(Flags {
                implicit: true,
                dotted,
                inline: scope == Scope::Inline,
            });

            py.put(from, &name, table.as_any())?;

            return Ok(Some(table));
        };

        match self.container(&existing) {
            Some(Container::Table(mut flags)) => {
                if scope == Scope::Document && flags.inline {
                    self.violate(
                        defer,
                        "cannot extend value of type inline table with a dotted key",
                        segment.span,
                    );

                    return Ok(None);
                }

                if dotted && flags.implicit {
                    flags.dotted = true;
                    self.note(&existing, Container::Table(flags));
                }

                if dotted && !flags.implicit {
                    self.violate(defer, "duplicate key", segment.span);

                    return Ok(None);
                }

                Ok(existing.cast_into::<PyDict>().ok())
            }
            Some(Container::Array { aot: true }) => {
                let Ok(list) = existing.cast_into::<PyList>() else {
                    return Ok(None);
                };
                let item = list.as_any().get_item(-1)?;

                if !matches!(self.container(&item), Some(Container::Table(_))) {
                    self.violate(
                        defer,
                        format!(
                            "cannot extend value of type {} with a dotted key",
                            type_str(&item)
                        ),
                        segment.span,
                    );

                    return Ok(None);
                }

                Ok(item.cast_into::<PyDict>().ok())
            }
            Some(Container::Array { aot: false }) => {
                self.violate(
                    defer,
                    "cannot extend value of type array with a dotted key",
                    segment.span,
                );

                Ok(None)
            }
            None => {
                self.violate(
                    defer,
                    format!(
                        "cannot extend value of type {} with a dotted key",
                        type_str(&existing)
                    ),
                    segment.span,
                );

                Ok(None)
            }
        }
    }

    #[inline]
    fn open_header(&mut self, is_array: bool) {
        self.header = Some(is_array);
        self.keys.clear();
    }

    fn apply_header(&mut self) -> PyResult<bool> {
        let Some(is_array) = self.header.take() else {
            return Ok(true);
        };
        self.flush_deferred();

        let keys = std::mem::take(&mut self.keys);

        if keys.is_empty() {
            return Ok(false);
        }

        let (parent_key, path) = keys.split_last().expect("keys is not empty");
        let root = self.root.clone();

        let Some(parent) = self.descend(root, path, false, Scope::Document, is_array)? else {
            if is_array {
                self.table = self.py.dict();

                return Ok(true);
            }

            return Ok(false);
        };
        let name = self.py.key(&parent_key.text);

        if is_array {
            let Some(list) = self.array_table(&parent, &name, parent_key.span)? else {
                self.table = self.py.dict();

                return Ok(true);
            };
            let table = self.new_table(Flags::default());

            self.py.push(&list, table.as_any())?;
            self.table = table;
        } else {
            let Some(table) = self.std_table(&parent, &name, parent_key.span)? else {
                return Ok(false);
            };

            self.table = table;
        }

        // keep the allocation for the next header
        let mut keys = keys;
        keys.clear();
        self.keys = keys;

        Ok(true)
    }

    fn std_table(
        &mut self,
        parent: &Bound<'py, PyDict>,
        name: &Bound<'py, PyString>,
        span: Span,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        let Some(existing) = self.py.get(parent, name)? else {
            let table = self.new_table(Flags::default());

            self.py.put(parent, name, table.as_any())?;

            return Ok(Some(table));
        };

        let flags = self.flags(&existing);

        if !flags.implicit || flags.dotted {
            self.report("duplicate key", span);

            return Ok(None);
        }

        self.note(&existing, Container::Table(Flags::default()));

        Ok(existing.cast_into::<PyDict>().ok())
    }

    fn array_table(
        &mut self,
        parent: &Bound<'py, PyDict>,
        name: &Bound<'py, PyString>,
        span: Span,
    ) -> PyResult<Option<Bound<'py, PyList>>> {
        if let Some(existing) = self.py.get(parent, name)? {
            if !matches!(
                self.container(&existing),
                Some(Container::Array { aot: true })
            ) {
                self.defer("duplicate key", span);

                return Ok(None);
            }

            return Ok(existing.cast_into::<PyList>().ok());
        }

        let list = self.new_list(true);

        self.py.put(parent, name, list.as_any())?;

        Ok(Some(list))
    }

    #[inline]
    fn start_key_value(&mut self) -> bool {
        let Some(key) = self.keys.pop() else {
            return false;
        };
        let path = if self.keys.is_empty() {
            Vec::new()
        } else {
            std::mem::take(&mut self.keys)
        };
        let slot = match self.targets.last() {
            Some(Target::Inline(table)) => Slot::Inline(table.clone()),
            _ => Slot::Document,
        };

        self.targets.push(Target::Slot { slot, path, key });

        true
    }

    #[inline]
    fn complete(&mut self, value: &Bound<'py, PyAny>) -> PyResult<bool> {
        if let Some(Target::Array(list)) = self.targets.last() {
            self.py.push(list, value)?;

            return Ok(true);
        }

        let Some(Target::Slot { slot, path, key }) = self.targets.pop() else {
            return Ok(false);
        };
        let scope = slot.scope();
        let dotted = scope == Scope::Inline || !path.is_empty();

        if path.is_empty() && matches!(slot, Slot::Document) {
            let name = self.py.key(&key.text);
            let inserted = self.py.put(&self.table, &name, value)?;

            if !inserted {
                self.report("duplicate key", key.span);

                return Ok(false);
            }

            return Ok(true);
        }

        let base = match slot {
            Slot::Document => self.table.clone(),
            Slot::Inline(table) => table,
        };

        let Some(parent) = self.descend(base, &path, dotted, scope, false)? else {
            return Ok(false);
        };

        let mixed = match scope {
            Scope::Document => dotted && !self.flags(parent.as_any()).implicit,
            Scope::Inline => self.flags(parent.as_any()).dotted == path.is_empty(),
        };

        if mixed {
            self.report("duplicate key", key.span);

            return Ok(false);
        }

        let name = self.py.key(&key.text);

        if !self.py.put(&parent, &name, value)? {
            self.report("duplicate key", key.span);

            return Ok(false);
        }

        Ok(true)
    }

    fn build_scalar(
        &mut self,
        kind: ScalarKind,
        decoded: &str,
        span: Span,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        match kind {
            ScalarKind::String => Ok(Some(self.py.string(decoded))),
            ScalarKind::Boolean(value) => Ok(Some(self.py.flag(value))),
            ScalarKind::Integer(radix) => self.build_integer(decoded, radix.value(), span),
            ScalarKind::Float => self.build_float(decoded, span),
            ScalarKind::DateTime => self.build_datetime(decoded, span),
        }
    }

    fn build_float(&mut self, decoded: &str, span: Span) -> PyResult<Option<Bound<'py, PyAny>>> {
        // fast path for default value in signature
        if self.parse_float.is(self.py.get_type::<PyFloat>())
            && let Ok(value) = decoded.parse::<f64>()
        {
            return Ok(Some(self.py.float(value)?));
        }

        Ok(self.custom_float(span))
    }

    #[cold]
    fn custom_float(&mut self, span: Span) -> Option<Bound<'py, PyAny>> {
        // the callback sees the document's own text, separators included
        let input = self.source.input();
        let literal = &input[span.start()..span.end().min(input.len())];

        let value = match self.parse_float.call1((literal,)) {
            Ok(value) => value,
            Err(error) => {
                self.failed(error);

                return None;
            }
        };

        // https://github.com/hukkin/tomli/blob/2.4.1/src/tomli/_parser.py#L789-L790
        if value.is_instance_of::<PyDict>() || value.is_instance_of::<PyList>() {
            self.failed(PyValueError::new_err(
                "parse_float must not return dicts or lists",
            ));

            return None;
        }

        Some(value)
    }

    fn build_integer(
        &mut self,
        decoded: &str,
        radix: u32,
        span: Span,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let bytes = decoded.as_bytes();
        let options = lexical_core::ParseIntegerOptions::new();

        if let Ok(value) = parse_int!(i64, bytes, &options, radix) {
            return Ok(Some(self.py.int(value)?));
        }

        if let Some(value) = BigInt::parse_bytes(bytes, radix) {
            return Ok(Some(value.into_bound_py_any(self.py)?));
        }

        let input = self.source.input();
        let span = span.start()..span.end().min(input.len());

        let literal = &input[span.start..span.end.min(input.len())];

        self.failed(
            DecodeError::snippet(&format!("invalid integer '{literal}'"), input, span)
                .raised(self.py),
        );

        Ok(None)
    }

    fn build_datetime(&mut self, decoded: &str, span: Span) -> PyResult<Option<Bound<'py, PyAny>>> {
        let py = self.py;
        let datetime = match decoded.parse::<toml_v1::value::Datetime>() {
            Ok(datetime) => datetime,
            Err(error) => {
                self.report(error.to_string(), span);

                return Ok(None);
            }
        };

        let value = match (datetime.date, datetime.time, datetime.offset) {
            (Some(date), Some(time), Some(offset)) => {
                let tzinfo = create_timezone_from_offset(py, offset)?;

                create_py_datetime_v1!(py, date, time, Some(&tzinfo))?.into_any()
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
            _ => return Ok(None),
        };

        Ok(Some(value))
    }
}

impl EventReceiver for RawReceiver<'_, '_, '_> {
    fn std_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        self.open_header(false);
    }

    fn std_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        let result = self.apply_header();

        self.finish(result);
    }

    fn array_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        self.open_header(true);
    }

    fn array_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        let result = self.apply_header();

        self.finish(result);
    }

    fn inline_table_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) -> bool {
        guard!(self => true);

        let table = self.new_table(Flags {
            implicit: false,
            dotted: false,
            inline: true,
        });

        self.targets.push(Target::Inline(table));

        true
    }

    fn inline_table_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        guard!(self);

        let Some(Target::Inline(table)) = self.targets.pop() else {
            self.bail = true;

            return;
        };
        let result = self.complete(&table.into_any());

        self.finish(result);
    }

    fn array_open(&mut self, _span: Span, _error: &mut dyn ErrorSink) -> bool {
        guard!(self => true);

        let list = self.new_list(false);

        self.targets.push(Target::Array(list));

        true
    }

    fn array_close(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        guard!(self);

        let Some(Target::Array(list)) = self.targets.pop() else {
            self.bail = true;

            return;
        };
        let result = self.complete(&list.into_any());

        self.finish(result);
    }

    fn simple_key(&mut self, span: Span, encoding: Option<Encoding>, _error: &mut dyn ErrorSink) {
        guard!(self);

        match self.decode_key(span, encoding) {
            Ok(text) => self.keys.push(Key { span, text }),
            Err(error) => self.report_error(error),
        }
    }

    fn key_val_sep(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        guard!(self);

        if !self.start_key_value() {
            self.bail = true;
        }
    }

    fn scalar(&mut self, span: Span, encoding: Option<Encoding>, _error: &mut dyn ErrorSink) {
        guard!(self);

        let (kind, decoded) = match self.decode_scalar(span, encoding) {
            Ok(decoded) => decoded,
            Err(error) => {
                self.report_error(error);

                return;
            }
        };

        let result = match self.build_scalar(kind, decoded.as_ref(), span) {
            Ok(Some(value)) => self.complete(&value),
            Ok(None) => Ok(false),
            Err(error) => Err(error),
        };

        self.finish(result);
    }

    fn error(&mut self, _span: Span, _error: &mut dyn ErrorSink) {
        // the parser reported the real error through the sink
        self.bail = true;
    }
}

pub fn load<'py>(
    py: Python<'py>,
    source: &str,
    parse_float: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyDict>> {
    let parsed = Source::new(source);
    let tokens = parsed.lex().into_vec();
    let root = py.dict();
    let mut receiver = RawReceiver {
        py,
        source: parsed,
        parse_float,
        root: root.clone(),
        table: root.clone(),
        targets: Vec::new(),
        keys: Vec::new(),
        containers: FxHashMap::default(),
        header: None,
        semantic: None,
        deferred: None,
        value_error: None,
        bail: false,
    };
    let mut errors = Option::<ParseError>::None;
    {
        let mut validated =
            ValidateWhitespace::new(&mut receiver as &mut dyn EventReceiver, parsed);

        toml_parser_v1::parser::parse_document(&tokens, &mut validated, &mut errors);
    }

    receiver.flush_deferred();

    if let Some(error) = errors {
        return Err(to_py_error(py, source, &error));
    }

    if let Some(error) = receiver.semantic {
        return Err(to_py_error(py, source, &error));
    }

    if let Some(error) = receiver.value_error {
        return Err(error);
    }

    if receiver.bail || receiver.header.is_some() || !receiver.targets.is_empty() {
        return Err(to_py_error(
            py,
            source,
            &ParseError::new("internal error: unfinished receiver state"),
        ));
    }

    Ok(root)
}
