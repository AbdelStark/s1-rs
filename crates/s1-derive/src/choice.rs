use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{DeriveInput, Error, parse_macro_input};

use crate::attr::{parse_enum_attr, parse_variant_attr};
use crate::rename;
use crate::util::{
    MAX_CHOICE_OPTIONS, MIN_CHOICE_OPTIONS, combine_errors, crate_path, debug_impl, expect_enum,
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
    reject_generics(input, "derive(Choice)")?;
    let data = expect_enum(input, "derive(Choice)")?;
    let enum_attr = parse_enum_attr(&input.attrs)?;
    let s1 = crate_path();
    let name = &input.ident;

    let n = data.variants.len();
    if n < MIN_CHOICE_OPTIONS {
        return Err(Error::new_spanned(
            name,
            format!("Choice requires at least {MIN_CHOICE_OPTIONS} variants"),
        ));
    }
    if n > MAX_CHOICE_OPTIONS {
        return Err(Error::new_spanned(
            name,
            format!("Choice supports at most {MAX_CHOICE_OPTIONS} variants"),
        ));
    }

    let rename_all = enum_attr.rename_all.as_deref().unwrap_or("snake_case");

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
    let mut labels = Vec::new();
    let mut descriptions = Vec::new();
    let mut indices = Vec::new();
    let mut seen_labels = HashSet::new();

    for (i, variant) in data.variants.iter().enumerate() {
        if let Err(e) = reject_fields(&variant.fields, "Choice") {
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
        let label = match vattr.label {
            Some(l) => l,
            None => match rename::apply(&ident.to_string(), rename_all, ident.span()) {
                Ok(l) => l,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            },
        };
        if !seen_labels.insert(label.clone()) {
            errors.push(Error::new_spanned(
                ident,
                format!("duplicate Choice label {label:?}"),
            ));
        }

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
                        "variant {ident} needs a description: #[s1(\"...\")] (or set allow_empty_descriptions)"
                    ),
                ));
                continue;
            }
        };

        var_idents.push(ident.clone());
        labels.push(label);
        descriptions.push(description);
        indices.push(i);
    }

    combine_errors(errors)?;

    let extra = extra_impls(name, false);
    let debug = debug_impl(name, &var_idents);

    Ok(quote! {
        #extra
        #debug

        impl #s1::ChoiceQuestion for #name {
            const INSTRUCTIONS: #s1::Instructions = #instructions_tokens;
            const VARIANTS: &'static [Self] = &[#(Self::#var_idents),*];

            fn label(self) -> &'static str {
                match self {
                    #(Self::#var_idents => #labels,)*
                }
            }

            fn description(self) -> #s1::Description {
                match self {
                    #(Self::#var_idents => #descriptions,)*
                }
            }

            fn from_label(label: &str) -> Option<Self> {
                match label {
                    #(#labels => Some(Self::#var_idents),)*
                    _ => None,
                }
            }

            fn index(self) -> usize {
                match self {
                    #(Self::#var_idents => #indices,)*
                }
            }
        }

        impl #s1::QuestionField for #name {
            type Answer = #s1::ChoiceAnswer<Self>;

            fn wire_question() -> #s1::Question {
                #s1::Question::from_choice::<Self>()
            }

            fn decode_field(
                key: &str,
                resp: &#s1::WireResponse,
                meta: &mut #s1::AnswerMeta,
            ) -> Result<Self::Answer, #s1::DecodeError> {
                #s1::ChoiceAnswer::<Self>::decode(key, resp, meta)
            }
        }
    })
}
