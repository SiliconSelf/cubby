//! Macros for use in the cubby server

use std::{any::TypeId, io::Write};

use proc_macro::TokenStream;
use syn::{
    parse::{Parse, ParseStream, Result},
    parse_macro_input, DeriveInput, Ident, LitStr, Token,
};

/// Contains a Key/Value pair from a helper attribute
///
/// [Thanks, Charles](https://gitlab.computer.surgery/charles/far/-/blob/5a8e928c045cddedb8866ab51c24e228e9417c8f/macros/src/lib.rs#L87-142)
/// Parses attributes of the typical form
struct AttrNameValue<V> {
    /// The name of this attribute argument
    ///
    /// ```ignore
    /// #[far(fmt = "{}")]
    /// //    ^^^
    /// ```
    name: Ident,

    /// The token separating this argument from its value
    ///
    /// ```ignore
    /// #[far(fmt = "{}")]
    /// //        ^
    /// ```
    #[allow(dead_code)]
    eq_token: Token![=],

    /// The value of this attribute argument
    ///
    /// ```ignore
    /// #[far(fmt = "{}")]
    /// //          ^^^^
    /// ```
    ///
    /// If `V` is not a [`LitStr`][LitStr], `V` will first be parsed into a
    /// [`LitStr`][LitStr] and then into the actual type of `V`. Otherwise, it
    /// will only be parsed once into [`LitStr`][LitStr]. This may sound weird
    /// but it behaves exactly how you'd expect.
    ///
    /// [LitStr]: struct@LitStr
    value: V,
}

impl<V> Parse for AttrNameValue<V>
where
    V: Parse + 'static,
{
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            name: input.parse()?,
            eq_token: input.parse()?,
            value: {
                // Fine, I'll just implement my own specialization
                if TypeId::of::<V>() == TypeId::of::<LitStr>() {
                    input.parse()?
                } else {
                    let x: LitStr = input.parse()?;
                    x.parse()?
                }
            },
        })
    }
}

/// Derive macro for the IntoMatrixError trait
#[proc_macro_derive(IntoMatrixError, attributes(matrix_error))]
pub fn derive_into_matrix_error(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let mut file = std::fs::File::create("macro_output.txt").unwrap();
    file.write_all(format!("{:#?}", input.clone()).as_bytes())
        .expect("Failed to write log");
    // Assertions that guarantee we actually can derive thiw
    match input.data {
        syn::Data::Struct(_) | syn::Data::Union(_) => {
            panic!("IntoMatrixError can only be derived for enums")
        }
        syn::Data::Enum(e) => {
            let mut variants = Vec::new();
            for v in e.variants {
                variants.push(v);
            }
        }
    }

    TokenStream::new()
}
