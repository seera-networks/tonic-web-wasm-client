use http::{
    header::{ACCEPT, CONTENT_TYPE},
    response::Builder,
    HeaderMap, HeaderValue, Request, Response,
};
use http_body_util::BodyExt;
use pyo3::{
    intern,
    prelude::*,
    sync::GILOnceCell,
    types::{IntoPyDict, PyBytes, PyDict, PyString},
};
use tonic::body::BoxBody;

use crate::{pyodide_js::fetch::fetch, Error, ResponseBody};

pub async fn call(
    mut base_url: String,
    request: Request<BoxBody>,
) -> Result<Response<ResponseBody>, Error> {
    base_url.push_str(&request.uri().to_string());

    let headers = Python::with_gil(|py| -> Result<_, Error> {
        Ok(prepare_headers(py, request.headers())?.unbind())
    })?;
    let body = prepare_body(request).await?;
    let request = Python::with_gil(|py| -> Result<_, Error> {
        Ok(prepare_request(
            py,
            &base_url,
            headers.bind(py),
            body.as_ref().map(|x| x.bind(py)),
        )?
        .unbind())
    })?;

    let response = fetch(&request).await?;

    let (result, content_type, body_reader) = Python::with_gil(|py| -> Result<_, Error> {
        let response = response.bind(py);
        let result = Response::builder().status(
            response
                .getattr(intern!(py, "status"))
                .map_err(Error::py_error)?
                .extract::<u16>()
                .map_err(Error::py_error)?,
        );
        let (result, content_type) = set_response_headers(py, result, &response)?;
        let body_reader = response
            .getattr(intern!(py, "body"))
            .map_err(Error::py_error)?
            .call_method0(intern!(py, "getReader"))
            .map_err(Error::py_error)?
            .unbind();
        Ok((result, content_type, body_reader))
    })?;

    let content_type = content_type.ok_or(Error::MissingContentTypeHeader)?;

    let body = ResponseBody::new(body_reader, &content_type)?;

    result.body(body).map_err(Into::into)
}

fn prepare_headers<'py>(
    py: Python<'py>,
    header_map: &HeaderMap<HeaderValue>,
) -> Result<Bound<'py, PyDict>, Error> {
    let headers = PyDict::new_bound(py);

    headers
        .set_item(CONTENT_TYPE.as_str(), "application/grpc-web+proto")
        .map_err(Error::py_error)?;
    headers
        .set_item(ACCEPT.as_str(), "application/grpc-web+proto")
        .map_err(Error::py_error)?;
    headers
        .set_item("x-grpc-web", "1")
        .map_err(Error::py_error)?;

    for (header_name, header_value) in header_map.iter() {
        if header_name != CONTENT_TYPE && header_name != ACCEPT {
            headers
                .set_item(header_name.as_str(), header_value.to_str()?)
                .map_err(Error::py_error)?;
        }
    }

    Ok(headers)
}

async fn prepare_body(request: Request<BoxBody>) -> Result<Option<Py<PyBytes>>, Error> {
    let body = Some(request.collect().await?.to_bytes());
    Python::with_gil(|py| Ok(body.map(|bytes| PyBytes::new_bound(py, bytes.as_ref()).unbind())))
}

fn prepare_request<'py>(
    py: Python<'py>,
    url: &str,
    headers: &Bound<'py, PyDict>,
    body: Option<&Bound<'py, PyBytes>>,
) -> Result<Bound<'py, PyAny>, Error> {
    let init = PyDict::new_bound(py);
    init.set_item("method", "POST").map_err(Error::py_error)?;
    init.set_item("headers", headers).map_err(Error::py_error)?;
    init.set_item("body", body).map_err(Error::py_error)?;

    static NEW_REQUEST: GILOnceCell<Py<PyAny>> = GILOnceCell::new();
    let new_request = NEW_REQUEST
        .get_or_try_init(py, || -> PyResult<Py<PyAny>> {
            Ok(py
                .import_bound(intern!(py, "js"))?
                .getattr(intern!(py, "Request"))?
                .getattr(intern!(py, "new"))?
                .unbind())
        })
        .map(|x| x.bind(py))
        .map_err(Error::py_error)?;
    new_request
        .call1((url, to_js_object_bound(py, &init).map_err(Error::py_error)?))
        .map_err(Error::py_error)
}

fn set_response_headers<'py>(
    py: Python<'py>,
    mut result: Builder,
    response: &Bound<'py, PyAny>,
) -> Result<(Builder, Option<String>), Error> {
    static FROM_ENTRIES: GILOnceCell<Py<PyAny>> = GILOnceCell::new();
    let from_entries = FROM_ENTRIES
        .get_or_try_init(py, || -> PyResult<Py<PyAny>> {
            Ok(py
                .import_bound(intern!(py, "js"))?
                .getattr(intern!(py, "Object"))?
                .getattr(intern!(py, "fromEntries"))?
                .unbind())
        })
        .map(|x| x.bind(py))
        .map_err(Error::py_error)?;

    let headers = from_entries
        .call1((response
            .getattr(intern!(py, "headers"))
            .map_err(Error::py_error)?
            .call_method0(intern!(py, "entries"))
            .map_err(Error::py_error)?,))
        .map_err(Error::py_error)?
        .call_method0(intern!(py, "to_py"))
        .map_err(Error::py_error)?
        .downcast_into::<PyDict>()
        .map_err(|err| Error::py_error(err.into()))?;

    let mut content_type = None;

    for (header_name, header_value) in headers.iter() {
        if header_name.is_none() || header_value.is_none() {
            continue;
        }
        let header_name = header_name
            .downcast_into::<PyString>()
            .map_err(|err| Error::py_error(err.into()))?;
        let header_name = header_name.to_cow().map_err(Error::py_error)?;
        let header_value = header_value
            .downcast_into::<PyString>()
            .map_err(|err| Error::py_error(err.into()))?;
        let header_value = header_value.to_cow().map_err(Error::py_error)?;

        if header_name == CONTENT_TYPE.as_str() {
            content_type = Some(header_value.to_string());
        }
        result = result.header(header_name.as_ref(), header_value.as_ref());
    }

    Ok((result, content_type))
}

fn to_js_object_bound<'py>(
    py: Python<'py>,
    obj: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    static TO_JS_OBJECT: GILOnceCell<(Py<PyAny>, Py<PyAny>)> = GILOnceCell::new();
    let (to_js, from_entries) = TO_JS_OBJECT
        .get_or_try_init(py, || -> PyResult<(Py<PyAny>, Py<PyAny>)> {
            Ok((
                py.import_bound(intern!(py, "pyodide"))?
                    .getattr(intern!(py, "ffi"))?
                    .getattr(intern!(py, "to_js"))?
                    .unbind(),
                py.import_bound(intern!(py, "js"))?
                    .getattr(intern!(py, "Object"))?
                    .getattr(intern!(py, "fromEntries"))?
                    .unbind(),
            ))
        })
        .map(|(x, y)| (x.bind(py), y.bind(py)))?;
    let kwargs = vec![("dict_converter", from_entries)];
    to_js.call((obj,), Some(&kwargs.into_py_dict_bound(py)))
}

fn to_js_object<'py>(obj: &PyObject) -> PyResult<PyObject> {
    Python::with_gil(|py| to_js_object_bound(py, obj.bind(py)).map(|x| x.unbind()))
}
