// https://github.com/toml-rs/toml/blob/v0.24.0/crates/toml_edit/src/error.rs
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TomlError {
    message: String,
    input: Option<String>,
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

    pub fn set_input(&mut self, input: Option<&str>) {
        self.input = input.map(ToString::to_string);
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
    let line = input[0..line_start].iter().filter(|b| **b == b'\n').count();

    let column = std::str::from_utf8(&input[line_start..=index])
        .map(|s| s.chars().count() - 1)
        .unwrap_or_else(|_| index - line_start);
    let column = column + column_offset;

    (line, column)
}

impl fmt::Display for TomlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut context = false;
        if let (Some(input), Some(span)) = (&self.input, &self.span) {
            context = true;

            let (line, column) = translate_position(input.as_bytes(), span.start);
            let line_num = line + 1;
            let col_num = column + 1;
            let gutter = line_num.to_string().len();
            let content = input.split('\n').nth(line).expect("valid line number");
            let highlight_len = span.end - span.start;
            let highlight_len = highlight_len.min(content.len().saturating_sub(column));

            writeln!(f, "TOML parse error at line {line_num}, column {col_num}")?;
            for _ in 0..=gutter {
                write!(f, " ")?;
            }
            writeln!(f, "|")?;

            write!(f, "{line_num} | ")?;
            writeln!(f, "{content}")?;

            for _ in 0..=gutter {
                write!(f, " ")?;
            }
            write!(f, "|")?;
            for _ in 0..=column {
                write!(f, " ")?;
            }
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
