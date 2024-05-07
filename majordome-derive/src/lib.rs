mod majordome_errors;

/// Derive macro for the `IntoMajordomeError` trait.
/// Convert an enum to a MajordomeError.
/// Enum Attributes:
/// - `prefix`: Prefix for the error code. Required.
/// Enum Variants Attributes:
/// - `code`: Error code. Required.
/// - `msg`: Error message. The string is formatted using enum variant fields. Required.
/// - `status`: HTTP status code. Required.
///
/// # Example
/// ```rs
/// #[derive(MajordomeError)]
/// #[err(prefix = "errors.gg.wls.")]
/// pub enum AuthError {
///     #[err(code="invalid_token", msg="Invalid token", status=401)]
///     InvalidToken,
///
///     #[err(code="unknown_event", msg="Unknown event {id}", status=404)]
///     UnknownEvent {id: String},
///
///     #[err(code="not_enough_players", msg="Not enough players (required: {required}, actual: {actual})", status=400)]
///     NotEnoughPlayers{required: u32, actual: u32},
/// }
/// ```
///
/// Into/From are implemented for MajordomeError.
/// ```rs
/// AuthError::UnknownEvent{id: "123".to_string()}.into()
/// ```
/// equals to
/// ```rs
/// MajordomeError::new(
///     "errors.gg.wls.unknown_event".to_string(),
///     "Unknown event 123".to_string(),
///     vec!["123".to_string()],
///     404
/// )
/// ``````
#[proc_macro_derive(IntoMajordomeError, attributes(err))]
pub fn into_majordome_error_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    majordome_errors::parse_enum_error(input)
}
