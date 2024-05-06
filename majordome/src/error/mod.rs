use std::backtrace::Backtrace;
use uuid::Uuid;

#[derive(Debug, serde::Serialize, Clone)]
#[non_exhaustive]
pub struct MajordomeError {
    pub error: String,
    pub message: String,
    pub values: Vec<String>,
    #[serde(skip_serializing)]
    pub status_code: u16,
}

impl MajordomeError {
    pub fn err<T>(self) -> Result<T, Self> {
        Err(self)
    }

    pub fn new(error: String, message: String, values: Vec<String>, status_code: u16) -> Self {
        MajordomeError {
            error,
            message,
            values,
            status_code,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct InternalError {
    pub id: Uuid,
    pub inner: Box<dyn std::error::Error>,
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for MajordomeError {
    fn from(error: E) -> Self {
        let error_id = Uuid::new_v4();
        let error = InternalError {
            id: error_id.clone(),
            inner: Box::new(error),
        };

        // temporary: log the error
        tracing::error!("{:?}", error);

        MajordomeError {
            error: "errors.generic.internal".to_string(),
            message: format!("Something went wrong. Our team has been informed. (Error ID: {})", error_id),
            values: vec![error_id.to_string()],
            status_code: 500,
        }
    }
}

#[macro_export]
macro_rules! raise {
    ($error:expr) => {
        return $error.err();
    };
}

#[macro_export]
macro_rules! ensure {
    ($condition:expr, $error:expr) => {
        if !$condition {
            return $error.err();
        }
    };
}


// Converters
#[cfg(feature = "axum")]
mod axum;