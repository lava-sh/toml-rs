use memchr::{memchr, memchr_iter, memrchr};

#[derive(Clone)]
pub(crate) struct DocIndex<'a> {
    pub(crate) doc: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> DocIndex<'a> {
    pub(crate) fn new(doc: &'a str) -> Self {
        let mut line_starts = Vec::new();
        line_starts.push(0);
        for i in memchr_iter(b'\n', doc.as_bytes()) {
            line_starts.push(i + 1);
        }
        Self { doc, line_starts }
    }

    pub(crate) fn line_col(&self, pos: usize) -> (usize, usize) {
        let pos = pos.min(self.doc.len());
        let idx = self
            .line_starts
            .binary_search(&pos)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let line = idx + 1;
        let col = pos - self.line_starts[idx] + 1;
        (line, col)
    }

    pub(crate) fn find_open_back_to_line(&self, ch: u8, pos: usize) -> usize {
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

    pub(crate) fn find_table_header_end(&self, start: usize, is_array: bool) -> usize {
        let bytes = self.doc.as_bytes();
        if bytes.is_empty() {
            return 0;
        }

        let start = start.min(bytes.len().saturating_sub(1));
        let line_end = match memchr(b'\n', &bytes[start..]) {
            Some(i) => start + i,
            None => bytes.len(),
        };

        if is_array {
            let mut i = start;
            while i + 1 < line_end {
                if bytes[i] == b']' && bytes[i + 1] == b']' {
                    return i + 2;
                }
                i += 1;
            }
            line_end
        } else {
            match memchr(b']', &bytes[start..line_end]) {
                Some(i) => start + i + 1,
                None => line_end,
            }
        }
    }

    pub(crate) fn col_range_same_line(&self, start: usize, end: usize) -> (usize, usize) {
        let (_, c1) = self.line_col(start);
        let end_pos = end.saturating_sub(1).min(self.doc.len().saturating_sub(1));
        let (_, c2) = self.line_col(end_pos);
        (c1, c2)
    }

    pub(crate) fn value_line_range(&self, start: usize, end: usize) -> (usize, usize) {
        let (l1, _) = self.line_col(start);
        let end_pos = end.saturating_sub(1).min(self.doc.len().saturating_sub(1));
        let (l2, _) = self.line_col(end_pos);
        (l1, l2)
    }

    pub(crate) fn value_col_range_first_line(&self, start: usize, end: usize) -> (usize, usize) {
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
