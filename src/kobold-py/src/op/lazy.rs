use std::{ptr::NonNull, sync::Arc};

use kobold_object_property::value::{List, Object, Value};
use pyo3::{
    exceptions::{PyIndexError, PyKeyError},
    prelude::*,
};

use super::conversion::value_to_python;

#[derive(Clone)]
#[pyclass]
pub struct LazyList(Arc<Value>, NonNull<List>);

impl LazyList {
    // SAFETY: `current` must be derived from `base` in some way.
    pub unsafe fn new(base: Arc<Value>, current: &List) -> Self {
        Self(base, NonNull::from(current))
    }

    #[inline(always)]
    fn get_ref(&self) -> &List {
        // SAFETY: Constructor ensures our list is fine and we never get a mut ref.
        unsafe { self.1.as_ref() }
    }
}

// SAFETY: Raw pointers are never exposed for mutation.
unsafe impl Send for LazyList {}

#[pymethods]
impl LazyList {
    pub fn __len__(&self) -> usize {
        let list = self.get_ref();
        list.len()
    }

    pub fn __getitem__(&self, py: Python<'_>, idx: usize) -> PyResult<PyObject> {
        let list = self.get_ref();

        list.get(idx)
            .map(|v| unsafe { value_to_python(self.0.clone(), v, py) })
            .ok_or_else(|| PyIndexError::new_err("list index out of range"))
    }
}

#[derive(Clone)]
#[pyclass]
pub struct LazyObject(Arc<Value>, NonNull<Object>);

impl LazyObject {
    // SAFETY: `current` must be derived from `base` in some way.
    pub unsafe fn new(base: Arc<Value>, current: &Object) -> Self {
        Self(base, NonNull::from(current))
    }

    #[inline(always)]
    fn get_ref(&self) -> &Object {
        // SAFETY: Constructor ensures our list is fine and we never get a mut ref.
        unsafe { self.1.as_ref() }
    }
}

#[pymethods]
impl LazyObject {
    pub fn __len__(&self) -> usize {
        let obj = self.get_ref();
        obj.len()
    }

    pub fn __contains__(&self, key: &str) -> bool {
        let obj = self.get_ref();
        obj.contains_key(key)
    }

    pub fn __getitem__(&self, py: Python<'_>, key: &str) -> PyResult<PyObject> {
        let obj = self.get_ref();

        obj.get(key)
            .map(|v| unsafe { value_to_python(self.0.clone(), v, py) })
            .ok_or_else(|| PyKeyError::new_err(key.to_string()))
    }
}

// SAFETY: Raw pointers are never exposed for mutation.
unsafe impl Send for LazyObject {}
