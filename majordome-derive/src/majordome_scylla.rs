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
                                        Some(ident) if ident == "map" || ident == "counter" || ident == "set" => {
                                            fieldsettings.is_map = true;
                                            fieldsettings.kind = ident.to_string();
                                        },
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

    println!("{:?}", tablesettings);
    println!("{:?}", fields);

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

        let genmod = quote! {
            mod #modname {
                use ::scylla::frame::value::LegacySerializedValues;
                use super::*;

                pub struct #updatername<'a> {
                    inner: &'a mut #structname,
                    operations: Vec<u32>,
                    values: LegacySerializedValues
                }

                impl<'a> #updatername<'a> {
                    pub fn new(inner: &'a mut #structname) -> Self {
                        Self {
                            inner,
                            operations: Vec::new(),
                            values: LegacySerializedValues::new()
                        }
                    }

                    fn get_hash(&self) -> u64 {
                        use std::collections::hash_map::DefaultHasher;
                        use std::hash::{Hash, Hasher};
            
                        let mut hasher = DefaultHasher::new();
                        TypeId::of::<Self>().hash(&mut hasher);
                        self.operations.hash(&mut hasher);
                        hasher.finish()
                    }

                    fn generate_update_query(&self) -> String {
                        let q = self.operations.iter().map(|opcode| {
                            #mqq
                        }).collect::<Vec<&str>>().join(", ");
            
                        format!("UPDATE {} SET {} WHERE {}", #table, q, #updatewhereclause)
                    }

                    pub async fn save(self, scylla: &::majordome_scylla::ScyllaDB) -> Result<(), ::scylla::Error> {
                        let prepared = scylla.prepare_query_by_hash(self.get_hash(), || {
                            self.generate_update_query()
                        }).await?;

                        scylla.execute(prepared, self.values).await
                    }

                    #methods
                }
            }
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
            let mut opcode = branches.len();

            branches.push(quote! {
                #opcode => format!("{} = ?", #fieldname)
            });

            if field.is_map {
                opcode += 1;
                branches.push(quote! {
                    #opcode => format!("{} = {} + ?", #fieldname)
                });
                opcode += 1;
                branches.push(quote! {
                    #opcode => format!("{} = {} - ?", #fieldname)
                });
            }
        }

        quote! {
            match opcode {
                #(#branches),*,
                _ => panic!("Invalid opcode")
            }
        }.into()
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
                    self.inner.#fieldname = value;
                    self.operations.push(#opcode);
                    self.values.add_value(value);
                    self
                }
            });

            if field.is_map {
                opcode += 1;
                let methodname_add = quote::format_ident!("{}_add", fieldname);
                let modifier = match &field.kind[..] {
                    "map" => quote! {
                        self.inner.#fieldname.extend(value);
                    },
                    "counter" => quote! {
                        self.inner.#fieldname += value;
                    },
                    "set" => quote! {
                        for v in value {
                            self.inner.#fieldname.insert(v);
                        }
                    },
                    _ => quote! {}
                };
                methods.push(quote! {
                    pub fn #methodname_add(&mut self, value: #typename) -> &mut Self {
                        #modifier
                        self.operations.push(#opcode);
                        self.values.add_value(value);
                        self
                    }
                });

                opcode += 1;
                let methodname_rem = quote::format_ident!("{}_rem", fieldname);
                let modifier = match &field.kind[..] {
                    "map" => quote! {
                        for (k, v) in value {
                            self.inner.#fieldname.remove(&k);
                        }
                    },
                    "counter" => quote! {
                        self.inner.#fieldname -= value;
                    },
                    "set" => quote! {
                        for v in value {
                            self.inner.#fieldname.remove(&v);
                        }
                    },
                    _ => quote! {}
                };
                methods.push(quote! {
                    pub fn #methodname_rem(&mut self, value: #typename) -> &mut Self {
                        #modifier
                        self.operations.push(#opcode);
                        self.values.add_value(value);
                        self
                    }
                });
            }
        }

        quote! {
            #(#methods)*
        }
    }
}