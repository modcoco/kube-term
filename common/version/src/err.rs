// Make our own error that wraps `anyhow::Error`.
pub struct AxumErr(anyhow::Error);

// Convert AxumErr into axum response.
impl axum::response::IntoResponse for AxumErr {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AxumErr>`. That way you don't need to do that manually.
impl<E> From<E> for AxumErr
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

use axum::{
    http::{header::ToStrError, StatusCode},
    response::{IntoResponse, Response},
};
use thiserror::Error;
use tracing::error;
// use vaultrs::sys::ServerStatus;

///handler for error in the http service
///it convert the recevied error in a response
/// https://github.com/w6d-io/docker-opa
#[derive(Error, Debug)]
pub enum RouterError {
    #[error("failed to serialize data")]
    Serialisation(#[from] serde_json::Error),
    #[error("failed to apply identity patch")]
    Internal(#[from] anyhow::Error),
    #[error("failled to convert to string")]
    StrConvert(#[from] ToStrError),
    #[error("should never be empty.")]
    EmptyResponse,
}

#[cfg(not(tarpaulin_include))]
impl IntoResponse for RouterError {
    fn into_response(self) -> Response {
        match self {
            RouterError::Serialisation(e) => {
                error!("{:?}", e);
                #[cfg(test)]
                let status_string = format!("INTERNAL_SERVER_ERROR {e}");
                #[cfg(not(test))]
                let status_string = "INTERNAL_SERVER_ERROR";
                (StatusCode::INTERNAL_SERVER_ERROR, status_string).into_response()
            }
            RouterError::Internal(e) => {
                error!("{:?}", e);
                #[cfg(test)]
                let status_string = format!("INTERNAL_SERVER_ERROR {e}");
                #[cfg(not(test))]
                let status_string = "INTERNAL_SERVER_ERROR";
                (StatusCode::INTERNAL_SERVER_ERROR, status_string).into_response()
            }
            RouterError::StrConvert(e) => {
                error!("{:?}, while converting str", e);
                #[cfg(test)]
                let status_string = format!("INTERNAL_SERVER_ERROR {e}");
                #[cfg(not(test))]
                let status_string = "INTERNAL_SERVER_ERROR";
                (StatusCode::INTERNAL_SERVER_ERROR, status_string).into_response()
            }
            RouterError::EmptyResponse => {
                error!("opa returned empty response");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_SERVER_ERROR").into_response()
            }
        }
    }
}
