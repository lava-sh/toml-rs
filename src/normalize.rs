use std::borrow::Cow;

// https://github.com/python/cpython/blob/3.14/Lib/tomllib/_parser.py#L142
// The spec allows converting "\r\n" to "\n", even in string literals.
#[must_use]
pub(crate) fn normalize_line_ending(s: &'_ str) -> Cow<'_, str> {
    if memchr::memchr(b'\r', s.as_bytes()).is_none() {
        return Cow::Borrowed(s);
    }

    let finder = memchr::memmem::Finder::new(b"\r\n");

    let mut buf = s.to_string().into_bytes();
    let mut gap_len = 0;
    let mut tail = buf.as_mut_slice();

    loop {
        let idx = finder
            .find(&tail[gap_len..])
            .map_or(tail.len(), |idx| idx + gap_len);

        tail.copy_within(gap_len..idx, 0);

        tail = &mut tail[idx - gap_len..];

        if tail.len() == gap_len {
            break;
        }
        gap_len += 1;
    }

    // Account for removed `\r`.
    let new_len = buf.len() - gap_len;
    unsafe {
        // SAFETY: After `set_len`, `buf` is guaranteed to contain utf-8 again.
        buf.set_len(new_len);
        Cow::Owned(String::from_utf8_unchecked(buf))
    }
}
