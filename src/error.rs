use std::{fmt::Write as _, ops::Range};

use pyo3::{PyErr, Python, types::PyString};

use crate::toml_rs::TOMLDecodeError;

pub struct DecodeError<'doc> {
    rendered: String,
    doc: &'doc str,
    pos: usize,
}

impl<'doc> DecodeError<'doc> {
    pub fn snippet(message: &str, doc: &'doc str, span: Range<usize>) -> Self {
        let rendered = render(message, doc, &span);

        Self {
            rendered,
            doc,
            pos: span.start,
        }
    }

    pub fn raw(message: impl Into<String>, doc: &'doc str, pos: usize) -> Self {
        Self {
            rendered: message.into(),
            doc,
            pos,
        }
    }

    /// Raises it as `toml_rs.TOMLDecodeError`.
    #[must_use]
    pub fn raised(self, py: Python<'_>) -> PyErr {
        let (lineno, colno) = line_column(self.doc, self.pos);

        TOMLDecodeError::new_err((
            self.rendered,
            PyString::new(py, self.doc).unbind(),
            self.pos,
            lineno,
            colno,
        ))
    }
}

pub fn render(message: &str, input: &str, span: &Range<usize>) -> String {
    let bytes = input.as_bytes();
    let index = span.start.min(bytes.len().saturating_sub(1));
    let column_offset = span.start - index;
    let line_start = memchr::memrchr(b'\n', &bytes[..index]).map_or(0, |newline| newline + 1);
    let line = bytecount::count(&bytes[..line_start], b'\n');

    let column = if input.is_char_boundary(index + 1) {
        let end = index + 1;
        let head = &input[line_start..end];
        // ASCII is the common case, and then chars are bytes
        let chars = if head.is_ascii() {
            head.len()
        } else {
            head.chars().count()
        };

        chars - 1
    } else {
        index - line_start
    };
    let column = column + column_offset;

    let line_end = memchr::memchr(b'\n', &bytes[line_start..])
        .map_or(bytes.len(), |newline| line_start + newline);
    let content = &input[line_start..line_end];
    let line_num = line + 1;
    let col_num = column + 1;
    // one column of gutter, plus the `|` and the space before the line
    let gutter = decimal_len(line_num) + 1;
    let carets = (span.end - span.start).min(content.len().saturating_sub(column));

    let mut rendered =
        String::with_capacity(message.len() + content.len() + column + carets + gutter * 2 + 24);

    let _ = writeln!(
        rendered,
        "TOML parse error at line {line_num}, column {col_num}"
    );
    pad(&mut rendered, b' ', gutter);
    rendered.push_str("|\n");
    let _ = write!(rendered, "{line_num} | ");
    rendered.push_str(content);
    rendered.push('\n');
    pad(&mut rendered, b' ', gutter);
    rendered.push('|');
    pad(&mut rendered, b' ', column + 1);
    // the span is empty at eof, so there is always at least one `^`
    rendered.push('^');
    pad(&mut rendered, b'^', carets.saturating_sub(1));
    rendered.push('\n');
    rendered.push_str(message);
    rendered.push('\n');

    rendered
}

#[inline]
pub fn pad(out: &mut String, fill: u8, mut count: usize) {
    const CHUNK: usize = 64;

    let bytes = [fill; CHUNK];
    let Ok(block) = std::str::from_utf8(&bytes) else {
        return;
    };

    while count > CHUNK {
        out.push_str(block);
        count -= CHUNK;
    }

    out.push_str(&block[..count]);
}

const fn decimal_len(mut value: usize) -> usize {
    let mut digits = 1;

    while value >= 10 {
        value /= 10;
        digits += 1;
    }

    digits
}

#[inline]
pub fn line_column(input: &str, pos: usize) -> (Option<usize>, Option<usize>) {
    let bytes = input.as_bytes();
    let prefix = &bytes[..pos.min(bytes.len())];

    if !prefix.is_ascii() {
        return (None, None);
    }

    let newlines = bytecount::count(prefix, b'\n');

    if newlines == 0 {
        // `lineno == 1` is the one case where the column is the offset itself
        return (Some(1), Some(pos + 1));
    }

    let last = memchr::memrchr(b'\n', prefix).unwrap_or(0);

    (Some(newlines + 1), Some(pos - last))
}
