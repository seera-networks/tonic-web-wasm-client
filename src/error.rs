use http::header::{InvalidHeaderName, InvalidHeaderValue, ToStrError};
#[cfg(feature = "js")]
use js_sys::Object;
use thiserror::Error;
#[cfg(feature = "js")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(feature = "pyodide-js")]
use pyo3::PyErr;

/// Error type for `tonic-web-wasm-client`
#[derive(Debug, Error)]
pub enum Error {
    /// Base64 decode error
    #[error("base64 decode error")]
    Base64DecodeError(#[from] base64::DecodeError),
    /// Header parsing error
    #[error("failed to parse headers")]
    HeaderParsingError,
    /// Header value error
    #[error("failed to convert header value to string")]
    HeaderValueError(#[from] ToStrError),
    /// HTTP error
    #[error("http error")]
    HttpError(#[from] http::Error),
    /// Invalid content type
    #[error("invalid content type: {0}")]
    InvalidContentType(String),
    /// Invalid header name
    #[error("invalid header name")]
    InvalidHeaderName(#[from] InvalidHeaderName),
    /// Invalid header value
    #[error("invalid header value")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
    #[cfg(feature = "js")]
    /// JS API error
    #[error("js api error: {0}")]
    JsError(String),
    #[cfg(feature = "pyodide-js")]
    /// Python API error
    #[error("py api error: {0}")]
    PyError(String),
    /// Malformed response
    #[error("malformed response")]
    MalformedResponse,
    /// Missing `content-type` header in gRPC response
    #[error("missing content-type header in grpc response")]
    MissingContentTypeHeader,
    /// Missing response body in HTTP call
    #[error("missing response body in HTTP call")]
    MissingResponseBody,
    /// gRPC error
    #[error("grpc error")]
    TonicStatusError(#[from] tonic::Status),
}

impl Error {
    #[cfg(feature = "js")]
    /// Initialize js error from js value
    pub(crate) fn js_error(value: JsValue) -> Self {
        let message = js_object_display(&value);
        Self::JsError(message)
    }

    #[cfg(feature = "pyodide-js")]
    /// Initialize py error from PyErr
    pub(crate) fn py_error(value: PyErr) -> Self {
        Self::PyError(value.to_string())
    }
}

#[cfg(feature = "js")]
fn js_object_display(option: &JsValue) -> String {
    let object: &Object = option.unchecked_ref();
    ToString::to_string(&object.to_string())
}
