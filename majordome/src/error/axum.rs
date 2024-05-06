use axum::Json;

use crate::MajordomeError;

impl axum::response::IntoResponse for MajordomeError {
    fn into_response(self) -> axum::http::Response<axum::body::Body> {
        let status = axum::http::StatusCode::from_u16(self.status_code).unwrap();
        (status, Json(self)).into_response()
    }
}