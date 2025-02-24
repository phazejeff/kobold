use kobold_object_property::serde::Error as OpError;
use kobold_wad::ArchiveError;
use pyo3::prelude::*;

use crate::KoboldError;

pub fn op_to_py_err(err: OpError) -> PyErr {
    match err {
        OpError::Io(e) => e.into(),
        e => KoboldError::new_err(format!("{e:?}")),
    }
}

pub fn wad_to_py_err(err: ArchiveError) -> PyErr {
    match err {
        ArchiveError::Io(e) => e.into(),
        e => KoboldError::new_err(format!("{e:?}")),
    }
}
