// https://github.com/toml-rs/toml/blob/v0.24.0/crates/toml_edit/src/error.rs
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TomlError {
    message: String,
    input: Option<std::sync::Arc<str>>,
    keys: Vec<String>,
    span: Option<std::ops::Range<usize>>,
}

impl TomlError {
    pub fn custom(message: String, span: Option<std::ops::Range<usize>>) -> Self {
        Self {
            message,
            input: None,
            keys: Vec::new(),
            span,
        }
    }

    /// The start/end index into the original document where the error occurred
    pub fn span(&self) -> Option<std::ops::Range<usize>> {
        self.span.clone()
    }

    pub(crate) fn set_input(&mut self, input: Option<&str>) {
        self.input = input.map(Into::into);
    }
}

// Displays a TOML parse error
//
// # Example
//
// TOML parse error at line 1, column 10
//   |
// 1 | 00:32:00.a999999
//   |          ^
// Unexpected `a`
// Expected `digit`
// While parsing a Time
// While parsing a Date-Time
impl std::fmt::Display for TomlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut context = false;
        if let (Some(input), Some(span)) = (&self.input, self.span()) {
            context = true;

            let (line, column) = translate_position(input.as_bytes(), span.start);
            let line_num = line + 1;
            let col_num = column + 1;
            let gutter = line_num.to_string().len();
            let content = input.split('\n').nth(line).expect("valid line number");
            let highlight_len = span.end - span.start;
            // Allow highlight to go one past the line
            let highlight_len = highlight_len.min(content.len().saturating_sub(column));

            writeln!(f, "TOML parse error at line {line_num}, column {col_num}")?;
            //   |
            for _ in 0..=gutter {
                write!(f, " ")?;
            }
            writeln!(f, "|")?;

            // 1 | 00:32:00.a999999
            write!(f, "{line_num} | ")?;
            writeln!(f, "{content}")?;

            //   |          ^
            for _ in 0..=gutter {
                write!(f, " ")?;
            }
            write!(f, "|")?;
            for _ in 0..=column {
                write!(f, " ")?;
            }
            // The span will be empty at eof, so we need to make sure we always print at least
            // one `^`
            write!(f, "^")?;
            for _ in 1..highlight_len {
                write!(f, "^")?;
            }
            writeln!(f)?;
        }
        writeln!(f, "{}", self.message)?;
        if !context && !self.keys.is_empty() {
            writeln!(f, "in `{}`", self.keys.join("."))?;
        }

        Ok(())
    }
}

fn translate_position(input: &[u8], index: usize) -> (usize, usize) {
    if input.is_empty() {
        return (0, index);
    }

    let safe_index = index.min(input.len().saturating_sub(1));
    let column_offset = index - safe_index;
    let index = safe_index;

    let nl = input[0..index]
        .iter()
        .rev()
        .enumerate()
        .find(|(_, b)| **b == b'\n')
        .map(|(nl, _)| index - nl - 1);

    let line_start = match nl {
        Some(nl) => nl + 1,
        None => 0,
    };

    let line = bytecount::count(&input[0..line_start], b'\n');

    let column = std::str::from_utf8(&input[line_start..=index])
        .map_or_else(|_| index - line_start, |s| s.chars().count() - 1);

    let column = column + column_offset;

    (line, column)
}
