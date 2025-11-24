use pyo3::{PyResult, exceptions::PyRecursionError};

#[derive(Copy, Clone, Debug)]
struct Limit(usize);

impl Limit {
    #[inline]
    fn value_within_limit(self, value: usize) -> bool {
        value < self.0
    }
}

const RECURSION_LIMIT: Limit = Limit(999);

#[derive(Clone, Debug)]
pub(crate) struct RecursionGuard {
    current: usize,
    limit: Limit,
}

impl Default for RecursionGuard {
    fn default() -> Self {
        Self {
            current: 0,
            limit: RECURSION_LIMIT,
        }
    }
}

impl RecursionGuard {
    #[inline]
    pub fn enter(&mut self) -> PyResult<()> {
        if !self.limit.value_within_limit(self.current) {
            return Err(PyRecursionError::new_err("max recursion depth met"));
        }
        self.current += 1;
        Ok(())
    }

    #[inline]
    pub fn exit(&mut self) {
        self.current -= 1;
    }
}
