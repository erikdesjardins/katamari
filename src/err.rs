use axum::response::IntoResponse;
use hyper::StatusCode;
use std::fmt::{self, Debug, Write};

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct ResponseError(Error);

impl<T> From<T> for ResponseError
where
    T: Into<Error>,
{
    fn from(e: T) -> Self {
        ResponseError(e.into())
    }
}

impl Debug for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl IntoResponse for ResponseError {
    fn into_response(self) -> axum::response::Response {
        let mut body = String::from("Error: ");
        write!(body, "{}", self.0).unwrap();
        let mut err: &dyn std::error::Error = self.0.as_ref();
        while let Some(source) = err.source() {
            write!(body, " -> {}", source).unwrap();
            err = source;
        }

        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}
