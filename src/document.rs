use pyo3::{
    Bound, Py, PyAny, PyResult, Python,
    exceptions::{PyKeyError, PyTypeError},
    prelude::PyAnyMethods,
    pyclass, pymethods,
    types::PyDict,
};

#[pyclass]
pub struct TOMLDocument {
    #[pyo3(get)]
    pub value: Py<PyAny>,
    #[pyo3(get)]
    pub meta: Py<PyAny>,
}

fn parse_key_path(path: &str) -> Option<Vec<String>> {
    let source = toml_parser::Source::new(path);
    let mut errors = Vec::new();
    let keys = toml_edit::parse_key_path(source, &mut errors);

    if !errors.is_empty() {
        return None;
    }

    Some(keys.into_iter().map(|k| k.get().to_string()).collect())
}

#[pymethods]
impl TOMLDocument {
    fn __getitem__<'py>(
        &self,
        py: Python<'py>,
        key: Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let val = self.value.bind(py);

        if let Ok(s) = key.extract::<&str>() {
            if let Ok(item) = val.get_item(s) {
                return Ok(item);
            }

            if let Some(parts) = parse_key_path(s) {
                if parts.len() == 1 {
                    let part = &parts[0];
                    if let Ok(item) = val.get_item(part.as_str()) {
                        return Ok(item);
                    }
                } else if parts.len() > 1 {
                    let mut cur = val.clone();
                    for part in parts {
                        cur = cur
                            .get_item(part.as_str())
                            .map_err(|_| PyKeyError::new_err(s.to_string()))?;
                    }
                    return Ok(cur);
                }
            }

            return Err(PyKeyError::new_err(s.to_string()));
        }

        val.get_item(key)
    }

    fn __setitem__<'py>(
        &self,
        py: Python<'py>,
        key: Bound<'py, PyAny>,
        value: Bound<'py, PyAny>,
    ) -> PyResult<()> {
        let val = self.value.bind(py);

        if let Ok(s) = key.extract::<&str>() {
            if val.get_item(s).is_ok() {
                val.set_item(s, &value)?;
                return Ok(());
            }

            if let Some(parts) = parse_key_path(s) {
                if parts.len() == 1 {
                    let part = &parts[0];
                    val.set_item(part.as_str(), &value)?;
                    return Ok(());
                } else if parts.len() > 1 {
                    let mut cur = val.clone();
                    let mut it = parts.iter().peekable();

                    while let Some(part) = it.next() {
                        if it.peek().is_none() {
                            cur.set_item(part.as_str(), &value)?;
                            return Ok(());
                        }

                        if let Ok(next) = cur.get_item(part.as_str()) {
                            if !next.is_instance_of::<PyDict>() {
                                return Err(PyTypeError::new_err(format!(
                                    "Can't set dotted key '{s}': '{part}' is not a dict"
                                )));
                            }
                            cur = next;
                        } else {
                            let d = PyDict::new(py);
                            cur.set_item(part.as_str(), &d)?;
                            cur = d.into_any();
                        }
                    }
                }
            }

            val.set_item(s, &value)?;
            return Ok(());
        }

        val.set_item(key, value)?;
        Ok(())
    }

    fn __delitem__<'py>(&self, py: Python<'py>, key: Bound<'py, PyAny>) -> PyResult<()> {
        let val = self.value.bind(py);

        if let Ok(s) = key.extract::<&str>() {
            if matches!(val.del_item(s), Ok(())) {
                return Ok(());
            }

            if let Some(parts) = parse_key_path(s) {
                if parts.len() == 1 {
                    let part = &parts[0];
                    val.del_item(part.as_str())?;
                    return Ok(());
                } else if parts.len() > 1 {
                    let mut cur = val.clone();
                    let mut it = parts.iter().peekable();

                    while let Some(part) = it.next() {
                        if it.peek().is_none() {
                            cur.del_item(part.as_str())
                                .map_err(|_| PyKeyError::new_err(s.to_string()))?;
                            return Ok(());
                        }

                        cur = cur
                            .get_item(part.as_str())
                            .map_err(|_| PyKeyError::new_err(s.to_string()))?;
                    }
                }
            }

            return Err(PyKeyError::new_err(s.to_string()));
        }

        val.del_item(key)?;
        Ok(())
    }
}
