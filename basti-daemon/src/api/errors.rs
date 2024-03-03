use axum::{http::StatusCode, response::IntoResponse, response::Response, Json};
use std::fmt::{Debug, Display};

pub enum ApiErrorKind {
    Internal,
    NotFound,
}

impl Default for ApiErrorKind {
    fn default() -> Self {
        Self::Internal
    }
}

impl Display for ApiErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Internal => "Internal Server Error",
                Self::NotFound => "Not Found",
            }
        )
    }
}

impl ApiErrorKind {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
        }
    }
}

pub struct ApiError {
    pub inner: anyhow::Error,
    pub kind: ApiErrorKind,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.inner)
    }
}

impl Debug for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.kind, self.inner)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.kind.status_code(), self.to_string()).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        Self {
            kind: ApiErrorKind::default(),
            inner: value,
        }
    }
}

pub type ApiResult<T> = Result<(StatusCode, Json<T>), ApiError>;
