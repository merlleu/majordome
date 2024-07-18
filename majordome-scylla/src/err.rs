use majordome_derive::IntoMajordomeError;
#[derive(IntoMajordomeError)]
#[err(prefix = "errors.db.scylla.")]
pub enum ScyllaORMError {
    #[err(code = "not_found", msg = "{} not found", status = 404)]
    NotFoundExpectedOne(String),
    #[err(
        code = "too_many_results",
        msg = "Too many {}, expected one but found {}.",
        status = 500
    )]
    TooManyResultsExpectedOne(String, usize),
}
