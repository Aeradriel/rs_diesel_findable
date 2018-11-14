extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate proc_macro2;
extern crate regex;
extern crate syn;

use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{DeriveInput, Field, Ident, Type};

#[proc_macro_attribute]
pub fn findable_by(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut string_input = input.to_string();
    let string_args = args.to_string();
    let ast: DeriveInput = syn::parse(input).unwrap();
    let fields: Vec<Field> = match ast.data {
        syn::Data::Enum(..) => panic!("#[findable_by] cannot be used with enums"),
        syn::Data::Union(..) => panic!("#[findable_by] cannot be used with unions"),
        syn::Data::Struct(ref body) => body.fields.iter().map(|f| f.clone()).collect(),
    };
    let struct_attributes = string_args.replace(" ", "").replace("\"", "");
    let struct_attributes: Vec<&str> = struct_attributes.split(",").collect();
    let struct_name = ast.ident;

    for struct_attribute in struct_attributes {
        let mut attr_type = "".to_string();
        let field: Vec<&Field> = fields
            .iter()
            .filter(|f| f.ident.clone().unwrap().to_string() == struct_attribute)
            .collect();

        if field.len() > 0 {
            let field = field[0];

            if let Type::Path(ref field_type) = field.ty {
                attr_type = field_type.path.segments[0].ident.to_string();
            }

            let func_name = Ident::new(&format!("find_by_{}", struct_attribute), Span::call_site());
            let struct_attribute = Ident::new(&struct_attribute, Span::call_site());
            let struct_attribute_col =
                Ident::new(&format!("{}_col", struct_attribute), Span::call_site());
            let attr_type = Ident::new(&attr_type, Span::call_site());
            let table_name = Ident::new(&get_table_name(string_input.clone()), Span::call_site());

            let find_by_func = quote! {
                impl #struct_name {
                    pub fn #func_name(attr: & #attr_type, conn: &PgConnection) -> Option<#struct_name> {
                        use schema::#table_name::dsl::#struct_attribute as #struct_attribute_col;

                        match #table_name::table.filter(#struct_attribute_col.eq(attr)).first(conn) {
                            Ok(res) => Some(res),
                            Err(_) => None,
                        }
                    }
                }
            };

            string_input.push_str(&find_by_func.to_string());
        } else {
            panic!(
                "Attribute {} not found in {}",
                struct_attribute, struct_name
            );
        }
    }

    string_input.parse().unwrap()
}

fn get_table_name(input: String) -> String {
    use regex::Regex;

    let re = Regex::new(r###"#\[table_name = "(.*)"\]"###).unwrap();
    let table_name_attr = input
        .lines()
        .skip_while(|line| !line.trim_left().starts_with("#[table_name ="))
        .next()
        .expect("Struct must be annotated with #[table_name = \"...\"]");

    if let Some(table_name) = re.captures(table_name_attr).unwrap().get(1) {
        table_name.as_str().to_string()
    } else {
        panic!("Malformed table_name attribute");
    }
}
