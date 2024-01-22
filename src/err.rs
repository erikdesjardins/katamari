use axum::response::IntoResponse;
use hyper::StatusCode;

pub struct Error(Box<dyn std::error::Error + Send + Sync + 'static>);

impl<T> From<T> for Error
where
    T: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    fn from(e: T) -> Self {
        Error(e.into())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {}", self.0),
        )
            .into_response()
    }
}
