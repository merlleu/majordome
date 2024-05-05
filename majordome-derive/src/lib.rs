mod majordome_errors;

#[proc_macro_derive(IntoMajordomeError, attributes(err))]
pub fn into_majordome_error_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    majordome_errors::parse_enum_error(input)
}
