use crate::MajordomeError;
use axum::{
    extract::rejection::JsonRejection, extract::FromRequest,
    response::IntoResponse,
};
use serde::Serialize;

/// Used for custom error rejections.
pub struct _MajordomeRejectionError(MajordomeError);

impl axum::response::IntoResponse for MajordomeError {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        let status = axum::http::StatusCode::from_u16(self.status_code).unwrap();
        (status, Json(self)).into_response()
    }
}

impl axum::response::IntoResponse for _MajordomeRejectionError {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        self.0.into_response()
    }
}



// JSON
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(_MajordomeRejectionError))]
pub struct Json<T>(pub T);

impl<T: Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> axum::response::Response {
        let Self(value) = self;
        axum::Json(value).into_response()
    }
}

impl From<JsonRejection> for _MajordomeRejectionError {
    fn from(rejection: JsonRejection) -> Self {
        let e = match rejection {
            JsonRejection::BytesRejection(e) => MajordomeError {
                error: format!("errors.generic.bad_request.json.bytes_rejection"),
                message: format!("Failed to read JSON body: {}", e),
                values: vec![e.to_string()],
                status_code: 400
            },
            JsonRejection::JsonDataError(e) => MajordomeError {
                error: format!("errors.generic.bad_request.json.data_error"),
                message: format!("Failed to deserialize JSON body: {}", e),
                values: vec![e.to_string()],
                status_code: 400
            },
            JsonRejection::JsonSyntaxError(e) => MajordomeError {
                error: format!("errors.generic.bad_request.json.syntax_error"),
                message: format!("Failed to parse JSON body: {}", e),
                values: vec![e.to_string()],
                status_code: 400
            },
            JsonRejection::MissingJsonContentType(_e) => MajordomeError {
                error: format!("errors.generic.bad_request.json.missing_content_type"),
                message: format!("Expected request with `Content-Type: application/json`"),
                values: vec![],
                status_code: 415
            },
            _ => MajordomeError {
                error: format!("errors.generic.bad_request.json.unknown"),
                message: format!("Unknown JSON error: {:?}", rejection),
                values: vec![rejection.to_string()],
                status_code: 400
            }
        };
        Self(e)
    }
}

// FORM
#[derive(FromRequest)]
#[from_request(via(axum::Form), rejection(_MajordomeRejectionError))]
pub struct Form<T>(pub T);

impl<T: Serialize> IntoResponse for Form<T> {
    fn into_response(self) -> axum::response::Response {
        let Self(value) = self;
        axum::Json(value).into_response()
    }
}

