mod majordome_errors;
mod majordome_scylla;

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
/// ```
#[proc_macro_derive(IntoMajordomeError, attributes(err))]
pub fn into_majordome_error_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    majordome_errors::parse_enum_error(input)
}

/// Derive macro for the `ScyllaRow` trait.
/// ORM for ScyllaDB.
/// Struct Attributes:
/// - `table`: Table name. Required.
/// - `primary_key`: Primary key field names. Required.
/// - `clustering_key`: Clustering key field names. Optional.
/// - `indexes`: Index field names. Optional.
/// 
/// Struct Fields Attributes:
/// - `map`: Map field. Optional.
/// - `set`: Set field. Optional.
/// - `counter`: Counter field. Optional.
/// 
/// # Example
/// ```rs
/// #[derive(ScyllaRow)]
/// #[scylla(table = "users", primary_key = "id")]
/// pub struct UserDBRepr {
///    pub id: i64,
///    pub email: Option<String>,
///    pub sponsor_id: Option<i64>,
///    pub p_desc: Option<String>,
///    #[scylla(map = 1)]
///    pub assets: std::collections::BTreeMap<String, String>,
///    pub flags: i64,
/// }
/// ```
/// 
/// This can then be used to update the struct.
/// ```rs
/// let u = UserDBRepr::new(1);
/// let u = u.update().email_set(Some("test".to_string())).assets_add({
///    let mut m = BTreeMap::new();
///    m.insert("test".to_string(), "test".to_string());
///    m
/// }).save().await?;
/// ```
#[proc_macro_derive(ScyllaRow, attributes(scylla))]
pub fn scylla_row_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    majordome_scylla::parse_struct_orm(input)
}
