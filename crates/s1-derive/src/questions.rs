use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::spanned::Spanned;
use syn::{DeriveInput, Error, Fields, Ident, Type, parse_macro_input};

use crate::attr::{ident_or_err, parse_enum_attr, parse_field_attr};
use crate::util::{combine_errors, crate_path, expect_struct, reject_generics};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => tokens.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

enum FieldKind {
    Typed {
        ty: Type,
        optional: bool,
    },
    Noul {
        instructions: String,
        true_means: Option<String>,
        false_means: Option<String>,
        optional: bool,
    },
}

struct QField {
    ident: Ident,
    key: String,
    kind: FieldKind,
}

pub fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    reject_generics(input, "derive(Questions)")?;
    let data = expect_struct(input, "derive(Questions)")?;
    let enum_attr = parse_enum_attr(&input.attrs)?;
    let s1 = crate_path();
    let name = &input.ident;
    let vis = &input.vis;

    let fields = match &data.fields {
        Fields::Named(f) => &f.named,
        _ => {
            return Err(Error::new_spanned(
                input,
                "#[derive(Questions)] requires a struct with named fields",
            ));
        }
    };

    let answers_name = match enum_attr.answers.as_deref() {
        Some(custom) => ident_or_err(custom, name.span())?,
        None => ident_or_err(&format!("{name}Answers"), name.span())?,
    };

    let mut errors = Vec::new();
    let mut qfields = Vec::new();
    let mut seen_keys = HashSet::new();

    for field in fields {
        let Some(ident) = field.ident.clone() else {
            errors.push(Error::new(field.span(), "unnamed field is not supported"));
            continue;
        };
        let fattr = match parse_field_attr(field) {
            Ok(a) => a,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        if fattr.flatten {
            errors.push(Error::new_spanned(
                field,
                "#[s1(flatten)] is not supported in s1 v0.1",
            ));
            continue;
        }
        let key = fattr.key.clone().unwrap_or_else(|| ident.to_string());
        if !seen_keys.insert(key.clone()) {
            errors.push(Error::new_spanned(
                &ident,
                format!("duplicate question key {key:?}"),
            ));
        }

        let (optional, inner) = strip_option(&field.ty);
        let kind = if is_bool(inner) {
            let Some(instructions) = fattr.noul else {
                errors.push(Error::new_spanned(
                    &ident,
                    "bool fields need #[s1(noul = \"...\")]",
                ));
                continue;
            };
            FieldKind::Noul {
                instructions,
                true_means: fattr.true_means,
                false_means: fattr.false_means,
                optional,
            }
        } else if is_primitive(inner) {
            errors.push(Error::new_spanned(
                &field.ty,
                "unsupported field type for #[derive(Questions)]; use a Choice, a Score, or bool with #[s1(noul)]",
            ));
            continue;
        } else {
            if fattr.noul.is_some() {
                errors.push(Error::new_spanned(
                    &ident,
                    "noul is only valid on bool fields",
                ));
            }
            FieldKind::Typed {
                ty: inner.clone(),
                optional,
            }
        };
        qfields.push(QField { ident, key, kind });
    }

    combine_errors(errors)?;

    let mut answer_fields = Vec::new();
    let mut inserts = Vec::new();
    let mut decodes = Vec::new();

    for q in &qfields {
        let ident = &q.ident;
        let key = &q.key;
        match &q.kind {
            FieldKind::Typed { ty, optional } => {
                if *optional {
                    answer_fields
                        .push(quote!(pub #ident: Option<<#ty as #s1::QuestionField>::Answer>));
                    inserts.push(quote! {
                        __q.insert(::std::string::String::from(#key), <#ty as #s1::QuestionField>::wire_question());
                    });
                    decodes.push(quote! {
                        #ident: <#ty as #s1::QuestionField>::decode_optional(#key, resp, &mut meta)?
                    });
                } else {
                    answer_fields.push(quote!(pub #ident: <#ty as #s1::QuestionField>::Answer));
                    inserts.push(quote! {
                        __q.insert(::std::string::String::from(#key), <#ty as #s1::QuestionField>::wire_question());
                    });
                    decodes.push(quote! {
                        #ident: <#ty as #s1::QuestionField>::decode_field(#key, resp, &mut meta)?
                    });
                }
            }
            FieldKind::Noul {
                instructions,
                true_means,
                false_means,
                optional,
            } => {
                let t = match true_means {
                    Some(s) => quote!(::core::option::Option::Some(#s1::Entry::from(#s))),
                    None => quote!(::core::option::Option::None),
                };
                let f = match false_means {
                    Some(s) => quote!(::core::option::Option::Some(#s1::Entry::from(#s))),
                    None => quote!(::core::option::Option::None),
                };
                inserts.push(quote! {
                    __q.insert(
                        ::std::string::String::from(#key),
                        #s1::Question::noul(#instructions, #t, #f),
                    );
                });
                if *optional {
                    answer_fields.push(quote!(pub #ident: Option<#s1::NoulAnswer>));
                    decodes.push(quote! {
                        #ident: if resp.answers.contains_key(#key) {
                            ::core::option::Option::Some(#s1::NoulAnswer::decode(#key, resp, &mut meta)?)
                        } else {
                            ::core::option::Option::None
                        }
                    });
                } else {
                    answer_fields.push(quote!(pub #ident: #s1::NoulAnswer));
                    decodes.push(quote! {
                        #ident: #s1::NoulAnswer::decode(#key, resp, &mut meta)?
                    });
                }
            }
        }
    }

    Ok(quote! {
        #[derive(Clone, Debug)]
        #vis struct #answers_name {
            #(#answer_fields,)*
            pub meta: #s1::AnswerMeta,
        }

        #[allow(dead_code)]
        impl #s1::QuestionSet for #name {
            type Answers = #answers_name;

            fn questions() -> #s1::WireQuestions {
                let mut __q = #s1::WireQuestions::new();
                #(#inserts)*
                __q
            }

            fn decode(resp: &#s1::WireResponse) -> Result<Self::Answers, #s1::DecodeError> {
                let mut meta = #s1::AnswerMeta::from_response(resp);
                Ok(#answers_name {
                    #(#decodes,)*
                    meta,
                })
            }
        }
    })
}

fn strip_option(ty: &Type) -> (bool, &Type) {
    if let Type::Path(p) = ty {
        if let Some(seg) = p.path.segments.last() {
            if seg.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return (true, inner);
                    }
                }
            }
        }
    }
    (false, ty)
}

fn is_bool(ty: &Type) -> bool {
    matches!(ty, Type::Path(p) if p.path.is_ident("bool"))
}

fn is_primitive(ty: &Type) -> bool {
    let Type::Path(p) = ty else {
        return false;
    };
    let Some(ident) = p.path.get_ident() else {
        return false;
    };
    matches!(
        ident.to_string().as_str(),
        "u8" | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
            | "char"
            | "str"
            | "String"
    )
}
