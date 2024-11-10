use core::future::Future;
use pyo3::{intern, prelude::*, sync::GILOnceCell, types::PyFuture};

use crate::Error;

fn js_fetch(
    request: &PyObject,
) -> PyResult<impl Future<Output = PyResult<PyObject>> + Send + Sync + 'static> {
    Python::with_gil(|py| {
        static FETCH: GILOnceCell<Py<PyAny>> = GILOnceCell::new();
        let fetch = FETCH
            .get_or_try_init(py, || -> PyResult<Py<PyAny>> {
                Ok(py
                    .import_bound(intern!(py, "js"))?
                    .getattr(intern!(py, "fetch"))?
                    .unbind())
            })
            .map(|x| x.bind(py))?;
        fetch
            .call1((request.bind(py),))?
            .downcast_into::<PyFuture>()?
            .as_rust_future()
    })
}

pub async fn fetch(request: &PyObject) -> Result<PyObject, Error> {
    js_fetch(request)
        .map_err(Error::py_error)?
        .await
        .map_err(Error::py_error)
}
