use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Result};

pub const MAX_CHOICE_OPTIONS: usize = 255;
pub const MIN_CHOICE_OPTIONS: usize = 2;
pub const MIN_SCORE_LEVELS: usize = 2;

pub fn crate_path() -> TokenStream {
    quote!(::s1)
}

pub fn expect_enum<'a>(input: &'a DeriveInput, derive: &str) -> Result<&'a syn::DataEnum> {
    match &input.data {
        Data::Enum(data) => Ok(data),
        _ => Err(Error::new_spanned(
            input,
            format!("#[{derive}] can only be used on enums"),
        )),
    }
}

pub fn expect_struct<'a>(input: &'a DeriveInput, derive: &str) -> Result<&'a syn::DataStruct> {
    match &input.data {
        Data::Struct(data) => Ok(data),
        _ => Err(Error::new_spanned(
            input,
            format!("#[{derive}] can only be used on structs"),
        )),
    }
}

pub fn reject_generics(input: &DeriveInput, derive: &str) -> Result<()> {
    if input.generics.params.is_empty() {
        Ok(())
    } else {
        Err(Error::new_spanned(
            &input.generics,
            format!("#[{derive}] does not support generic types"),
        ))
    }
}

pub fn reject_fields(fields: &Fields, what: &str) -> Result<()> {
    match fields {
        Fields::Unit => Ok(()),
        Fields::Named(f) => Err(Error::new(
            f.brace_token.span.join(),
            format!("{what} variants must be fieldless"),
        )),
        Fields::Unnamed(f) => Err(Error::new(
            f.paren_token.span.join(),
            format!("{what} variants must be fieldless"),
        )),
    }
}

pub fn extra_impls(name: &syn::Ident, score: bool) -> TokenStream {
    let mut impls = quote! {
        #[automatically_derived]
        impl ::core::marker::Copy for #name {}
        #[automatically_derived]
        impl ::core::clone::Clone for #name {
            fn clone(&self) -> Self { *self }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialEq for #name {
            fn eq(&self, other: &Self) -> bool {
                ::core::mem::discriminant(self) == ::core::mem::discriminant(other)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for #name {}
    };
    if score {
        let s1 = crate_path();
        impls.extend(quote! {
            #[automatically_derived]
            impl ::core::cmp::PartialOrd for #name {
                fn partial_cmp(&self, other: &Self) -> Option<::core::cmp::Ordering> {
                    Some(::core::cmp::Ord::cmp(self, other))
                }
            }
            #[automatically_derived]
            impl ::core::cmp::Ord for #name {
                fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                    #s1::ScoreQuestion::index(*self).cmp(&#s1::ScoreQuestion::index(*other))
                }
            }
        });
    }
    impls
}

pub fn debug_impl(name: &syn::Ident, variants: &[syn::Ident]) -> TokenStream {
    quote! {
        #[automatically_derived]
        impl ::core::fmt::Debug for #name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    #(Self::#variants => ::core::fmt::Formatter::write_str(f, stringify!(#variants)),)*
                }
            }
        }
    }
}

pub fn combine_errors(errors: Vec<Error>) -> Result<()> {
    let mut iter = errors.into_iter();
    let Some(mut first) = iter.next() else {
        return Ok(());
    };
    for e in iter {
        first.combine(e);
    }
    Err(first)
}
