use proc_macro::TokenStream;
use quote::*;
use syn::*;

#[derive(Debug)]
pub struct TableSettings {
    pub struct_name: String,
    pub table: Option<String>,
    pub indexes: Vec<String>,
    pub primary_key: Option<Vec<String>>,
    pub clustering_key: Vec<String>,
}

#[derive(Debug)]
pub struct FieldSettings {
    pub name: String,
    pub type_name: String,
    pub is_map: bool,
    pub kind: String,
    pub is_pk: bool,
}

pub(crate) fn parse_struct_orm(input: TokenStream) -> TokenStream {
    let ast: syn::DeriveInput = syn::parse(input).unwrap();

    let mut tablesettings = TableSettings {
        struct_name: ast.ident.to_string(),
        table: None,
        indexes: Vec::new(),
        primary_key: None,
        clustering_key: Vec::new(),
    };

    for attr in ast.attrs.iter() {
        if !attr.path.is_ident("scylla") {
            continue;
        }

        let meta = attr.parse_meta().unwrap();
        match meta {
            Meta::List(nv) => {
                for nested_meta in nv.nested {
                    if let NestedMeta::Meta(Meta::NameValue(nv)) = nested_meta {
                        match nv.path.get_ident() {
                            Some(ident) if ident == "table" => {
                                if let Lit::Str(lit_str) = nv.lit {
                                    tablesettings.table = Some(lit_str.value());
                                }
                            }
                            Some(ident) if ident == "indexes" => {
                                if let Lit::Str(lit_str) = nv.lit {
                                    tablesettings.indexes =
                                        lit_str.value().split(',').map(|s| s.to_string()).collect();
                                }
                            }
                            Some(ident) if ident == "primary_key" => {
                                if let Lit::Str(lit_str) = nv.lit {
                                    tablesettings.primary_key = Some(
                                        lit_str.value().split(',').map(|s| s.to_string()).collect(),
                                    );
                                }
                            }
                            Some(ident) if ident == "clustering_key" => {
                                if let Lit::Str(lit_str) = nv.lit {
                                    tablesettings.clustering_key =
                                        lit_str.value().split(',').map(|s| s.to_string()).collect();
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

    let mut fields = Vec::new();
    match ast.data {
        Data::Struct(struc) => {
            for field in struc.fields.iter() {
                let mut fieldsettings = FieldSettings {
                    name: field.ident.as_ref().unwrap().to_string(),
                    type_name: field.ty.to_token_stream().to_string(),
                    is_map: false,
                    kind: String::new(),
                    is_pk: false,
                };
                for attr in field.attrs.iter() {
                    if !attr.path.is_ident("scylla") {
                        continue;
                    }

                    let meta = attr.parse_meta().unwrap();

                    match meta {
                        Meta::List(nv) => {
                            for nested_meta in nv.nested {
                                if let NestedMeta::Meta(Meta::NameValue(nv)) = nested_meta {
                                    match nv.path.get_ident() {
                                        Some(ident)
                                            if ident == "map"
                                                || ident == "counter"
                                                || ident == "set" =>
                                        {
                                            fieldsettings.is_map = true;
                                            fieldsettings.kind = ident.to_string();
                                        }
                                        _ => (),
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }

                fields.push(fieldsettings);
            }
        }
        _ => {
            panic!("Attribute only applicable for structs")
        }
    }

    let table = tablesettings.table.expect("Table name not provided");
    let indexes = tablesettings.indexes;
    let primary_key = tablesettings.primary_key.expect("Primary key not provided");
    let clustering_key = tablesettings.clustering_key;

    for field in primary_key.iter() {
        let mut found = false;
        for f in fields.iter_mut() {
            if f.name == *field {
                f.is_pk = true;
                found = true;
            }
        }

        if !found {
            panic!("Primary key field not found in struct: {}", field);
        }
    }

    for field in clustering_key.iter() {
        let mut found = false;
        for f in fields.iter_mut() {
            if f.name == *field {
                f.is_pk = true;
                found = true;
            }
        }

        if !found {
            panic!("Clustering key field not found in struct: {}", field);
        }
    }

    let renderer = Renderer {
        table,
        indexes,
        primary_key,
        clustering_key,
        fields,
        struct_name: tablesettings.struct_name,
    };

    renderer.render()
}

struct Renderer {
    table: String,
    indexes: Vec<String>,
    primary_key: Vec<String>,
    clustering_key: Vec<String>,
    fields: Vec<FieldSettings>,
    struct_name: String,
}

impl Renderer {
    pub fn render(&self) -> TokenStream {
        let modname = quote::format_ident!("__mjscylla_{}", self.struct_name.to_lowercase());
        let structname = quote::format_ident!("{}", self.struct_name);
        let updatername = quote::format_ident!("{}Updater", structname);
        let mqq = self.render_match_query();
        let methods = self.render_methods();
        let table = &self.table;
        let updatewhereclause = self.render_update_where_clause();

        let newmethod = self.render_new_method();
        let updatermethod = self.render_update_method();
        let query = format!(
            "SELECT {} FROM {}",
            self.fields
                .iter()
                .map(|f| f.name.clone())
                .collect::<Vec<String>>()
                .join(", "),
            self.table
        );
        let select_methods = self.render_select_methods();
        let pktypes = self
            .fields
            .iter()
            .filter(|f| f.is_pk)
            .map(|f| syn::parse_str::<Type>(&f.type_name).unwrap())
            .collect::<Vec<_>>();

        let pk_add_quote = (0..pktypes.len()).map(
            |i| {
                let idx = syn::Index::from(i);
                quote!{ self.values.add_value(&self.pk.#idx).unwrap(); }
            }
        );
            

        let genmod = quote! {
            mod #modname {
                use ::scylla::frame::value::LegacySerializedValues;
                use super::*;

                pub struct #updatername{
                    pk: (#(#pktypes),*,),
                    operations: Vec<u32>,
                    values: LegacySerializedValues
                }

                impl #updatername {
                    pub fn new(pk: (#(#pktypes),*,), ) -> Self {
                        Self {
                            pk,
                            operations: Vec::new(),
                            values: LegacySerializedValues::new()
                        }
                    }

                    fn get_hash(&self) -> u64 {
                        use ::std::collections::hash_map::DefaultHasher;
                        use ::std::hash::{Hash, Hasher};

                        let mut hasher = DefaultHasher::new();
                        ::core::any::TypeId::of::<Self>().hash(&mut hasher);
                        self.operations.hash(&mut hasher);
                        hasher.finish()
                    }

                    fn generate_update_query(&self) -> String {
                        let q = self.operations.iter().map(|opcode| {
                            #mqq
                        }).collect::<Vec<&str>>().join(", ");

                        format!("UPDATE {} SET {} WHERE {}", #table, q, #updatewhereclause)
                    }

                    pub async fn save(&mut self, scylla: &::majordome_scylla::ScyllaDB) -> Result<(), ::scylla::transport::errors::QueryError> {
                        if self.operations.is_empty() {
                            return Ok(());
                        }
                        let prepared = scylla.prepare_by_hash_or(self.get_hash(), || {
                            self.generate_update_query()
                        }).await?;

                        #(#pk_add_quote)*
                        
                        let mut values = LegacySerializedValues::new();
                        std::mem::swap(&mut values, &mut self.values);
                        scylla.execute(&prepared, &values).await?;

                        self.operations = Vec::new();
                        Ok(())
                    }

                    pub fn is_saved(&self) -> bool {
                        self.operations.is_empty()
                    }
                    #methods
                }

                impl #structname {
                    #newmethod

                    #select_methods
                }

                impl ::majordome_scylla::ScyllaORMTable for #structname {
                    type Updater = #updatername;
                    #updatermethod

                    fn table_name() -> &'static str {
                        #table
                    }

                    fn query() -> &'static str {
                        #query
                    }
                }
            }
            pub use #modname::*;
        };

        genmod.into()
    }
    fn render_update_where_clause(&self) -> String {
        let mut keys = Vec::new();
        for key in self.primary_key.iter() {
            keys.push(format!("{} = ?", key));
        }

        for key in self.clustering_key.iter() {
            keys.push(format!("{} = ?", key));
        }

        keys.join(" AND ")
    }
    fn render_match_query(&self) -> proc_macro2::TokenStream {
        let mut branches = Vec::new();
        for field in self.fields.iter() {
            if field.is_pk {
                continue;
            }
            let fieldname = &field.name;
            let mut opcode = branches.len() as u32;

            let query_part = format!("{} = ?", fieldname);
            branches.push(quote! {
                #opcode => #query_part
            });

            if field.is_map {
                opcode += 1;
                let query_part = format!("{0} = {0} + ?", fieldname);
                branches.push(quote! {
                    #opcode => #query_part
                });
                opcode += 1;
                let query_part = format!("{0} = {0} - ?", fieldname);
                branches.push(quote! {
                    #opcode => #query_part
                });
            }
        }

        quote! {
            match opcode {
                #(#branches),*,
                _ => panic!("Invalid opcode")
            }
        }
        .into()
    }

    fn render_methods(&self) -> proc_macro2::TokenStream {
        let mut methods = Vec::new();
        for field in self.fields.iter() {
            if field.is_pk {
                continue;
            }
            let fieldname = quote::format_ident!("{}", field.name);
            let typename = syn::parse_str::<Type>(&field.type_name).unwrap();
            let methodname_set = quote::format_ident!("{}_set", fieldname);
            let mut opcode = methods.len() as u32;

            methods.push(quote! {
                pub fn #methodname_set(&mut self, value: #typename) -> &mut Self {
                    // self.inner.#fieldname = value.clone();
                    self.operations.push(#opcode);
                    self.values.add_value(&value).unwrap();
                    self
                }
            });

            if field.is_map {
                opcode += 1;
                let methodname_add = quote::format_ident!("{}_add", fieldname);
                // let modifier = match &field.kind[..] {
                //     "map" => quote! {
                //         self.inner.#fieldname.extend(value.clone());
                //     },
                //     "counter" => quote! {
                //         self.inner.#fieldname += value;
                //     },
                //     "set" => quote! {
                //         for v in value.clone() {
                //             self.inner.#fieldname.insert(v);
                //         }
                //     },
                //     _ => quote! {},
                // };
                methods.push(quote! {
                    pub fn #methodname_add(&mut self, value: #typename) -> &mut Self {
                        // #modifier
                        self.operations.push(#opcode);
                        self.values.add_value(&value).unwrap();
                        self
                    }
                });

                opcode += 1;
                let methodname_rem = quote::format_ident!("{}_rem", fieldname);
                let typename = match &field.kind[..] {
                    "map" => syn::parse_str::<Type>(&self.get_map_delete_key_ty(&field.type_name))
                        .unwrap(),
                    _ => typename.clone(),
                };
                // let modifier = match &field.kind[..] {
                //     "map" => quote! {
                //         for k in value.iter() {
                //             self.inner.#fieldname.remove(k);
                //         }
                //     },
                //     "counter" => quote! {
                //         self.inner.#fieldname -= value;
                //     },
                //     "set" => quote! {
                //         for v in value.iter() {
                //             self.inner.#fieldname.remove(v);
                //         }
                //     },
                //     _ => quote! {},
                // };
                methods.push(quote! {
                    pub fn #methodname_rem(&mut self, value: #typename) -> &mut Self {
                        // #modifier
                        self.operations.push(#opcode);
                        self.values.add_value(&value).unwrap();
                        self
                    }
                });
            }
        }

        quote! {
            #(#methods)*
        }
    }

    fn get_map_delete_key_ty(&self, ty: &str) -> String {
        let ty = ty.trim();
        let ty = ty
            .split('<')
            .last()
            .unwrap()
            .split(',')
            .next()
            .unwrap()
            .trim();
        format!("::std::vec::Vec<{}>", ty)
    }

    fn render_new_method(&self) -> proc_macro2::TokenStream {
        let mut fn_args = Vec::new();
        let mut fields = Vec::new();
        for field in self.fields.iter() {
            let fieldname = quote::format_ident!("{}", field.name);
            let typename = syn::parse_str::<Type>(&field.type_name).unwrap();

            if field.is_pk {
                fn_args.push(quote! {
                    #fieldname: #typename
                });
                fields.push(quote! {
                    #fieldname
                });
            } else {
                fields.push(quote! {
                    #fieldname: std::default::Default::default()
                });
            }
        }

        quote! {
            pub fn new(#(#fn_args),*) -> Self {
                Self {
                    #(#fields),*
                }
            }
        }
    }

    fn render_update_method(&self) -> proc_macro2::TokenStream {
        let mut fields = Vec::new();
        for field in self.fields.iter() {
            if !field.is_pk { continue; }
            let fieldname = quote::format_ident!("{}", field.name);
            fields.push(quote! {
                self.#fieldname.clone()
            });
        }

        quote! {
            fn update(&self) -> Self::Updater {
                let pk = (#(#fields),*,);
                Self::Updater::new(pk)
            }
        }
    }

    fn render_select_methods(&self) -> proc_macro2::TokenStream {
        let mut methods = vec![
            (self.primary_key.clone(), false), // (args, is_index)
        ];

        let mut args = self.primary_key.clone();
        for field in self.clustering_key.iter() {
            args.push(field.clone());
            methods.push((args.clone(), false));
        }

        for index in self.indexes.iter() {
            methods.push((vec![index.clone()], true));
        }

        let mut select_methods = Vec::new();
        for (args, is_index) in methods {
            let method = self.render_select_method(args, is_index);
            select_methods.push(method);
        }

        quote! {
            #(#select_methods)*
        }
    }

    fn render_select_method(&self, args: Vec<String>, is_index: bool) -> proc_macro2::TokenStream {
        let methodname = if is_index {
            quote::format_ident!("select_by_{}_index", args[0])
        } else {
            quote::format_ident!("select_by_{}", args.last().unwrap())
        };

        let mut whereclause = Vec::new();
        for arg in args.iter() {
            whereclause.push(format!("{} = ?", arg));
        }

        let whereclause = whereclause.join(" AND ");
        let query = format!(
            "SELECT {} FROM {} WHERE {}",
            self.fields
                .iter()
                .map(|f| f.name.clone())
                .collect::<Vec<String>>()
                .join(", "),
            self.table,
            whereclause
        );

        let mut nargs = Vec::new();
        let mut values = Vec::new();
        for arg in args.iter() {
            let mut typename = self
                .fields
                .iter()
                .find(|f| f.name == *arg)
                .unwrap()
                .type_name
                .clone();
            
            if typename.starts_with("Option < ") {
                typename = typename[9..typename.len()-2].to_string();
            }

            if typename == "String" {
                typename = "str".to_string();
            }

            let typename = syn::parse_str::<Type>(&typename).unwrap();
            let arg = quote::format_ident!("{}", arg);
            nargs.push(quote! {
                #arg: &#typename
            });
            values.push(quote! { #arg });
        }

        quote! {
            pub async fn #methodname(scylla: &::majordome_scylla::ScyllaDB, #(#nargs),*) -> Result<::majordome_scylla::MajordomeScyllaSelectResult<Self>, ::majordome::MajordomeError> {
                let rows = scylla.query(#query, (#(#values),*,)).await?;

                let mut resp = ::majordome_scylla::__private::smallvec::SmallVec::new();
                if let Some(rows) = rows.rows {
                    for row in rows {
                        resp.push(row.into_typed::<Self>()?);
                    }
                }

                Ok(::majordome_scylla::MajordomeScyllaSelectResult { resp })
            }
        }
    }
}
