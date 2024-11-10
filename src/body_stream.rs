use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::{stream::empty, Stream, TryStreamExt};
use http_body::{Body, Frame};
#[cfg(feature = "js")]
use js_sys::Uint8Array;
#[cfg(feature = "js")]
use wasm_streams::readable::IntoStream;
#[cfg(feature = "pyodide-js")]
use pyodide_js_stream::ReadableStream;

use crate::Error;

pub struct BodyStream {
    body_stream: Pin<Box<dyn Stream<Item = Result<Bytes, Error>> + Send + Sync + 'static>>,
}

impl BodyStream {
    #[cfg(all(feature = "js", not(feature = "pyodide-js")))]
    pub fn new(body_stream: IntoStream<'static>) -> Self {
        let body_stream = body_stream
            .map_ok(|js_value| {
                let buffer = Uint8Array::new(&js_value);

                let mut bytes_vec = vec![0; buffer.length() as usize];
                buffer.copy_to(&mut bytes_vec);

                bytes_vec.into()
            })
            .map_err(Error::js_error);

        Self {
            body_stream: Box::pin(body_stream),
        }
    }

    #[cfg(all(feature = "pyodide-js", not(feature = "js")))]
    pub fn new(body_stream: ReadableStream) -> Self {
        let body_stream = body_stream
            .map_ok(|bytes_vec| {
                bytes_vec.into()
            })
            .map_err(Error::py_error);

        Self {
            body_stream: Box::pin(body_stream),
        }
    }

    pub fn empty() -> Self {
        let body_stream = empty();

        Self {
            body_stream: Box::pin(body_stream),
        }
    }
}

impl Body for BodyStream {
    type Data = Bytes;

    type Error = Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        match self.body_stream.as_mut().poll_next(cx) {
            Poll::Ready(maybe) => Poll::Ready(maybe.map(|result| result.map(Frame::data))),
            Poll::Pending => Poll::Pending,
        }
    }
}

unsafe impl Send for BodyStream {}
unsafe impl Sync for BodyStream {}
