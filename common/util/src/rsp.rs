use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use common::{axum, reqwest, serde};
use reqwest::StatusCode;
use serde::Serialize;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Rsp<T> {
    code: u16,
    message: String,
    data: Option<T>,
    biz_status: Option<i32>,
    #[serde(skip_serializing)]
    http_status: StatusCode,
    #[serde(skip_serializing)]
    headers: Option<HeaderMap>,
}

impl<T> Rsp<T> {
    // Creates a successful response without any data
    pub fn success_without_data(message: &str) -> Self {
        Self {
            code: 0,
            message: message.to_string(),
            data: None,
            biz_status: None,
            http_status: StatusCode::OK,
            headers: None,
        }
    }

    // Creates a successful response with data
    pub fn success_with_data(data: T, message: &str) -> Self {
        Self {
            code: 0,
            message: message.to_string(),
            data: Some(data),
            biz_status: None,
            http_status: StatusCode::OK,
            headers: None,
        }
    }

    // Creates an error response with a custom code and message
    pub fn error(code: u16, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
            data: None,
            biz_status: None,
            http_status: StatusCode::INTERNAL_SERVER_ERROR,
            headers: None,
        }
    }

    // Creates an error response with a custom code, message, and business status
    pub fn error_with_biz_status(code: u16, message: &str, biz_status: i32) -> Self {
        Self {
            code,
            message: message.to_string(),
            data: None,
            biz_status: Some(biz_status),
            http_status: StatusCode::INTERNAL_SERVER_ERROR,
            headers: None,
        }
    }

    // Creates a success response with optional business status
    pub fn success_with_optional_biz_status(
        data: T,
        message: &str,
        biz_status: Option<i32>,
    ) -> Self {
        Self {
            code: 0,
            message: message.to_string(),
            data: Some(data),
            biz_status,
            http_status: StatusCode::OK,
            headers: None,
        }
    }

    // Set the HTTP status code manually
    pub fn with_http_status(mut self, status: StatusCode) -> Self {
        self.http_status = status;
        self
    }

    // Set headers manually
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }
}

impl<T> IntoResponse for Rsp<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let status = self.http_status;
        let headers = self.headers.clone();
        let body = axum::Json(self);

        let mut response = (status, body).into_response();

        if let Some(headers) = headers {
            *response.headers_mut() = headers;
        }

        response
    }
}
