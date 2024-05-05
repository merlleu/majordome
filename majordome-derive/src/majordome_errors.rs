use proc_macro::TokenStream;
use quote::*;
use syn::*;

pub(crate) fn parse_enum_error(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let mut prefix = String::new();
    for attr in ast.attrs.iter() {
        if !attr.path.is_ident("err") {
            continue;
        }

        let meta = attr.parse_meta().unwrap();
        match meta {
            Meta::List(nv) => {
                for nested_meta in nv.nested {
                    if let NestedMeta::Meta(Meta::NameValue(nv)) = nested_meta {
                        match nv.path.get_ident() {
                            Some(ident) if ident == "prefix" => {
                                if let Lit::Str(lit_str) = nv.lit {
                                    prefix = lit_str.value();
                                }
                            }
                            _ => (),
                        }
                    }
                }
            }
            _ => (),
        }
    }

    match ast.data {
        Data::Enum(DataEnum { variants, .. }) => {
            let name = &ast.ident;
            let enum_match_arms = variants.into_iter().map(|variant| {
                let variant_ident = &variant.ident;
                let attrs = &variant.attrs;
                let fields = &variant.fields;

                let mut code = String::new();
                let mut msg = String::new();
                let mut status: u16 = 0;

                for attr in attrs {
                    if attr.path.is_ident("err") {
                        let meta = attr.parse_meta().unwrap();
                        match meta {
                            Meta::List(nv) => {
                                for nested_meta in nv.nested {
                                    if let NestedMeta::Meta(Meta::NameValue(nv)) = nested_meta {
                                        match nv.path.get_ident() {
                                            Some(ident) if ident == "code" => {
                                                if let Lit::Str(lit_str) = nv.lit {
                                                    code = lit_str.value();
                                                }
                                            }
                                            Some(ident) if ident == "msg" => {
                                                if let Lit::Str(lit_str) = nv.lit {
                                                    msg = lit_str.value();
                                                }
                                            }
                                            Some(ident) if ident == "status" => {
                                                if let Lit::Int(lit_int) = nv.lit {
                                                    status = lit_int
                                                        .base10_parse()
                                                        .expect("status must be an u16");
                                                }
                                            }
                                            _ => (),
                                        }
                                    }
                                }
                            }
                            _ => (),
                        }
                    }
                }

                if code == "" {
                    panic!("Missing code attribute for variant {}", variant_ident);
                }
                if msg == "" {
                    panic!("Missing msg attribute for variant {}", variant_ident);
                }
                if status == 0 {
                    panic!("Missing status attribute for variant {}", variant_ident);
                }

                let code = format!("{}{}", prefix, code);

                match fields {
                    Fields::Named(FieldsNamed { named, .. }) => {
                        let field_names = named.iter().map(|f| &f.ident);
                        let field_names2 = field_names.clone();
                        let field_names3 = field_names.clone();
                        quote! {
                            #name::#variant_ident { #(#field_names),* } => {
                                ::majordome::MajordomeError::new(
                                    #code.to_string(),
                                    format!(#msg, #(#field_names2 = #field_names2),*),
                                    vec![#(#field_names3.to_string()),*],
                                    #status
                                )
                            }
                        }
                    }
                    Fields::Unit => {
                        quote! {
                            #name::#variant_ident => {
                                ::majordome::MajordomeError::new(
                                    #code.to_string(),
                                    #msg.to_string(),
                                    vec![],
                                    #status
                                )
                            }
                        }
                    }
                    Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
                        let field_names = unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, _)| quote::format_ident!("__p{}", i));
                        let field_names2 = field_names.clone();
                        let field_names3 = field_names.clone();
                        quote! {
                            #name::#variant_ident(#(#field_names),*) => {
                                ::majordome::MajordomeError::new(
                                    #code.to_string(),
                                    format!(#msg, #(#field_names2),*),
                                    vec![#(#field_names3.to_string()),*],
                                    #status
                                )
                            }
                        }
                    }
                }
            });

            let gen = quote! {
                impl From<#name> for ::majordome::MajordomeError {
                    fn from(err: #name) -> ::majordome::MajordomeError {
                        match err {
                            #(#enum_match_arms),*
                        }
                    }
                }

                impl #name {
                    /// Return Err(MajordomeError) with the current enum variant
                    /// as the error.
                    pub fn err<T>(self) -> Result<T, ::majordome::MajordomeError> {
                        Err(self.into())
                    }
                }
            };

            gen.into()
        }
        _ => {
            panic!("Attribute only applicable for enums")
        }
    }
}
