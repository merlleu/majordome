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
    pub backtrace: Box<Backtrace>,
    pub inner: Box<dyn std::error::Error>,
}

impl From<Box<dyn std::error::Error>> for MajordomeError {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        let error = InternalError {
            id: Uuid::new_v4(),
            backtrace: Box::new(Backtrace::force_capture()),
            inner: error,
        };

        // temporary: log the error
        log::error!("{:?}", error);

        MajordomeError {
            error: "errors.generic.internal".to_string(),
            message: format!("Something went wrong. Our team has been informed."),
            values: vec![],
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