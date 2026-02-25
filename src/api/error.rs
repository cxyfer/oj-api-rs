use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProblemDetail {
    #[serde(rename = "type")]
    pub error_type: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
}

#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl ProblemDetail {
    pub fn not_found(detail: impl Into<String>) -> Self {
        Self {
            error_type: "about:blank".into(),
            title: "Not Found".into(),
            status: 404,
            detail: detail.into(),
            errors: None,
        }
    }

    pub fn bad_request(detail: impl Into<String>) -> Self {
        Self {
            error_type: "about:blank".into(),
            title: "Bad Request".into(),
            status: 400,
            detail: detail.into(),
            errors: None,
        }
    }

    #[allow(dead_code)]
    pub fn validation(detail: impl Into<String>, errors: Vec<FieldError>) -> Self {
        Self {
            error_type: "about:blank".into(),
            title: "Validation Error".into(),
            status: 400,
            detail: detail.into(),
            errors: Some(errors),
        }
    }

    pub fn unauthorized(detail: impl Into<String>) -> Self {
        Self {
            error_type: "about:blank".into(),
            title: "Unauthorized".into(),
            status: 401,
            detail: detail.into(),
            errors: None,
        }
    }

    pub fn conflict(detail: impl Into<String>) -> Self {
        Self {
            error_type: "about:blank".into(),
            title: "Conflict".into(),
            status: 409,
            detail: detail.into(),
            errors: None,
        }
    }

    pub fn internal(detail: impl Into<String>) -> Self {
        Self {
            error_type: "about:blank".into(),
            title: "Internal Server Error".into(),
            status: 500,
            detail: detail.into(),
            errors: None,
        }
    }

    pub fn bad_gateway(detail: impl Into<String>) -> Self {
        Self {
            error_type: "about:blank".into(),
            title: "Bad Gateway".into(),
            status: 502,
            detail: detail.into(),
            errors: None,
        }
    }

    pub fn gateway_timeout(detail: impl Into<String>) -> Self {
        Self {
            error_type: "about:blank".into(),
            title: "Gateway Timeout".into(),
            status: 504,
            detail: detail.into(),
            errors: None,
        }
    }
}

impl IntoResponse for ProblemDetail {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = serde_json::to_string(&self).unwrap_or_default();
        (
            status,
            [(axum::http::header::CONTENT_TYPE, "application/problem+json")],
            body,
        )
            .into_response()
    }
}
