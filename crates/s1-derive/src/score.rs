use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, parse_macro_input};

use crate::attr::{parse_enum_attr, parse_variant_attr};
use crate::util::{
    MAX_CHOICE_OPTIONS, MIN_SCORE_LEVELS, combine_errors, crate_path, debug_impl, expect_enum,
    extra_impls, reject_fields, reject_generics,
};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    reject_generics(input, "derive(Score)")?;
    let data = expect_enum(input, "derive(Score)")?;
    let enum_attr = parse_enum_attr(&input.attrs)?;
    let s1 = crate_path();
    let name = &input.ident;

    let n = data.variants.len();
    if n < MIN_SCORE_LEVELS {
        return Err(Error::new_spanned(
            name,
            format!("Score requires at least {MIN_SCORE_LEVELS} levels"),
        ));
    }
    if n > MAX_CHOICE_OPTIONS {
        return Err(Error::new_spanned(
            name,
            format!("Score supports at most {MAX_CHOICE_OPTIONS} levels"),
        ));
    }

    let instructions_tokens = match (
        enum_attr.instructions.as_deref(),
        enum_attr.instructions_json.as_deref(),
    ) {
        (Some(text), None) => quote!(#s1::Instructions::Text(#text)),
        (None, Some(json)) => {
            if let Err(e) = serde_json::from_str::<serde_json::Value>(json) {
                return Err(Error::new_spanned(
                    name,
                    format!("invalid instructions_json: {e}"),
                ));
            }
            quote!(#s1::Instructions::Json(#json))
        }
        (Some(_), Some(_)) => {
            return Err(Error::new_spanned(
                name,
                "specify only one of instructions or instructions_json",
            ));
        }
        (None, None) => {
            return Err(Error::new_spanned(
                name,
                "missing #[s1(instructions = \"...\")]",
            ));
        }
    };

    let mut errors = Vec::new();
    let mut var_idents = Vec::new();
    let mut descriptions = Vec::new();
    let mut indices = Vec::new();

    for (i, variant) in data.variants.iter().enumerate() {
        if let Err(e) = reject_fields(&variant.fields, "Score") {
            errors.push(e);
            continue;
        }
        let vattr = match parse_variant_attr(variant) {
            Ok(v) => v,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        let ident = &variant.ident;
        let description = match (vattr.desc, vattr.desc_json) {
            (Some(d), None) => quote!(#s1::Description::Text(#d)),
            (None, Some(json)) => {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&json) {
                    errors.push(Error::new_spanned(ident, format!("invalid desc_json: {e}")));
                    continue;
                }
                quote!(#s1::Description::Json(#json))
            }
            (Some(_), Some(_)) => {
                errors.push(Error::new_spanned(
                    ident,
                    "specify only one of desc or desc_json",
                ));
                continue;
            }
            (None, None) if enum_attr.allow_empty_descriptions => {
                quote!(#s1::Description::Null)
            }
            (None, None) => {
                errors.push(Error::new_spanned(
                    ident,
                    format!(
                        "level {ident} needs a description: #[s1(\"...\")] (or set allow_empty_descriptions)"
                    ),
                ));
                continue;
            }
        };
        var_idents.push(ident.clone());
        descriptions.push(description);
        indices.push(i);
    }

    combine_errors(errors)?;

    let extra = extra_impls(name, true);
    let debug = debug_impl(name, &var_idents);
    let from_index_arms = indices
        .iter()
        .zip(var_idents.iter())
        .map(|(i, ident)| quote!(#i => Some(Self::#ident)));

    Ok(quote! {
        #extra
        #debug

        impl #s1::ScoreQuestion for #name {
            const INSTRUCTIONS: #s1::Instructions = #instructions_tokens;
            const LEVELS: &'static [Self] = &[#(Self::#var_idents),*];

            fn level_description(self) -> #s1::Description {
                match self {
                    #(Self::#var_idents => #descriptions,)*
                }
            }

            fn index(self) -> usize {
                match self {
                    #(Self::#var_idents => #indices,)*
                }
            }

            fn from_index(i: usize) -> Option<Self> {
                match i {
                    #(#from_index_arms,)*
                    _ => None,
                }
            }
        }

        impl #s1::QuestionField for #name {
            type Answer = #s1::ScoreAnswer<Self>;

            fn wire_question() -> #s1::Question {
                #s1::Question::from_score::<Self>()
            }

            fn decode_field(
                key: &str,
                resp: &#s1::WireResponse,
                meta: &mut #s1::AnswerMeta,
            ) -> Result<Self::Answer, #s1::DecodeError> {
                #s1::ScoreAnswer::<Self>::decode(key, resp, meta)
            }
        }
    })
}
