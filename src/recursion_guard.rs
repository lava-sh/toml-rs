#[derive(Copy, Clone, Debug)]
struct Limit(usize);

impl Limit {
    #[inline]
    fn _limit(&self, value: usize) -> bool {
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
    #[inline(always)]
    pub fn enter(&mut self) -> pyo3::PyResult<()> {
        if !self.limit._limit(self.current) {
            return Err(pyo3::exceptions::PyRecursionError::new_err(
                "max recursion depth met",
            ));
        }
        self.current += 1;
        Ok(())
    }

    #[inline(always)]
    pub fn exit(&mut self) {
        self.current -= 1;
    }
}
