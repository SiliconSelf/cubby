//! Macros for use in the cubby server

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    braced,
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    token, Attribute, Error, Generics, Ident, LitStr, Token, Variant,
    Visibility,
};

#[derive(Debug)]
struct NamedFieldsEnum {
    _attrs: Vec<Attribute>,
    _vis: Visibility,
    _enum_token: Token![enum],
    ident: Ident,
    generics: Generics,
    _brace_token: token::Brace,
    variants: Punctuated<Variant, Token![,]>,
}

impl Parse for NamedFieldsEnum {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Self {
            _attrs: input.call(Attribute::parse_outer)?,
            _vis: input.parse()?,
            _enum_token: input.parse()?,
            ident: input.parse()?,
            generics: input.parse()?,
            _brace_token: braced!(content in input),
            variants: content.parse_terminated(Variant::parse, Token![,])?,
        })
    }
}

struct IntoMatrixErrorArguments {
    http_status: Ident,
    _sep_1: Token![,],
    error_code: LitStr,
    _sep_2: Token![,],
    error_message: LitStr,
}

impl Parse for IntoMatrixErrorArguments {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            http_status: input.parse()?,
            _sep_1: input.parse()?,
            error_code: input.parse()?,
            _sep_2: input.parse()?,
            error_message: input.parse()?,
        })
    }
}

fn gen_insert(variant: &Variant) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;
    let fmt = variant
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("matrix_error"))
        .map(|attr| {
            
            attr.parse_args::<IntoMatrixErrorArguments>()
                .map_or_else(Error::into_compile_error, |attr| {
                    let status = attr.http_status;
                    let code = attr.error_code.value().to_string();
                    let message = attr.error_message.value().to_string();
                    quote! {
                        #variant_name => {
                            let errcode = #code;
                            let message = #message;
                            MatrixError {
                                status_code: axum::http::StatusCode::#status,
                                body: MatrixErrorBody::Json(json!({
                                    "errcode": errcode,
                                    "error": message
                            })),
                        }
                    }
                }})
        })
        .collect();
    fmt
}

/// Derive macro for the `IntoMatrixError` trait
#[proc_macro_derive(IntoMatrixError, attributes(matrix_error))]
pub fn derive_into_matrix_error(input: TokenStream) -> TokenStream {
    let named_fields = parse_macro_input!(input as NamedFieldsEnum);

    let enum_name = named_fields.ident;
    let enum_variants = named_fields.variants;

    let inserts: proc_macro2::TokenStream =
        enum_variants.iter().flat_map(gen_insert).collect();

    let (ig, tyg, _where_clause) = named_fields.generics.split_for_impl();

    quote! {
        impl #ig cubby_lib::IntoMatrixError for #enum_name #tyg {
            fn into_matrix_error(self) -> ruma::api::error::MatrixError {
                use ruma::api::error::{MatrixError, MatrixErrorBody};
                use serde_json::json;
                use #enum_name::*;
                match self {
                    #inserts
                }
            }
        }
    }
    .into()
}
