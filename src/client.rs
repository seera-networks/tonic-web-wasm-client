use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use http::{Request, Response};
use tonic::body::BoxBody;
use tower_service::Service;

#[cfg(feature = "js")]
use crate::js::call::call;
#[cfg(feature = "pyodide-js")]
use crate::pyodide_js::call::call;
use crate::{Error, ResponseBody};

/// `grpc-web` based transport layer for `tonic` clients
#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
}

impl Client {
    /// Creates a new client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
        }
    }

    // /// Creates a new client with options
    // pub fn new_with_options(base_url: String, options: FetchOptions) -> Self {
    //     Self {
    //         base_url,
    //         options: Some(options),
    //     }
    // }

    // /// Sets the options for the client
    // pub fn with_options(&mut self, options: FetchOptions) -> &mut Self {
    //     self.options = Some(options);
    //     self
    // }
}

impl Service<Request<BoxBody>> for Client {
    type Response = Response<ResponseBody>;

    type Error = Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<BoxBody>) -> Self::Future {
        // Box::pin(call(self.base_url.clone(), request, self.options.clone()))
        Box::pin(call(self.base_url.clone(), request))
    }
}
