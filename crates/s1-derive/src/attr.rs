use syn::spanned::Spanned;
use syn::{Attribute, Error, Expr, Field, Ident, LitStr, Meta, Result, Variant};

#[derive(Default)]
pub struct EnumAttr {
    pub instructions: Option<String>,
    pub instructions_json: Option<String>,
    pub rename_all: Option<String>,
    pub allow_empty_descriptions: bool,
    pub answers: Option<String>,
}

#[derive(Default)]
pub struct VariantAttr {
    pub label: Option<String>,
    pub desc: Option<String>,
    pub desc_json: Option<String>,
}

#[derive(Default)]
pub struct FieldAttr {
    pub noul: Option<String>,
    pub key: Option<String>,
    pub true_means: Option<String>,
    pub false_means: Option<String>,
    pub flatten: bool,
}

pub fn parse_enum_attr(attrs: &[Attribute]) -> Result<EnumAttr> {
    let mut out = EnumAttr::default();
    for attr in attrs {
        if !attr.path().is_ident("s1") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("instructions") {
                let value = meta.value()?;
                let s: LitStr = value.parse()?;
                out.instructions = Some(s.value());
                Ok(())
            } else if meta.path.is_ident("instructions_json") {
                let value = meta.value()?;
                let s: LitStr = value.parse()?;
                out.instructions_json = Some(s.value());
                Ok(())
            } else if meta.path.is_ident("rename_all") {
                let value = meta.value()?;
                let s: LitStr = value.parse()?;
                out.rename_all = Some(s.value());
                Ok(())
            } else if meta.path.is_ident("allow_empty_descriptions") {
                out.allow_empty_descriptions = true;
                Ok(())
            } else if meta.path.is_ident("answers") {
                let value = meta.value()?;
                let s: LitStr = value.parse()?;
                out.answers = Some(s.value());
                Ok(())
            } else {
                Err(meta.error("unrecognized s1 attribute"))
            }
        })?;
    }
    Ok(out)
}

pub fn parse_variant_attr(variant: &Variant) -> Result<VariantAttr> {
    let mut out = VariantAttr::default();
    for attr in &variant.attrs {
        if !attr.path().is_ident("s1") {
            continue;
        }
        match &attr.meta {
            Meta::List(list) => {
                if let Ok(s) = syn::parse2::<LitStr>(list.tokens.clone()) {
                    out.desc = Some(s.value());
                    continue;
                }
                list.parse_nested_meta(|meta| {
                    if meta.path.is_ident("label") {
                        let value = meta.value()?;
                        let s: LitStr = value.parse()?;
                        out.label = Some(s.value());
                        Ok(())
                    } else if meta.path.is_ident("desc") || meta.path.is_ident("description") {
                        let value = meta.value()?;
                        let s: LitStr = value.parse()?;
                        out.desc = Some(s.value());
                        Ok(())
                    } else if meta.path.is_ident("desc_json") {
                        let value = meta.value()?;
                        let s: LitStr = value.parse()?;
                        out.desc_json = Some(s.value());
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized s1 variant attribute"))
                    }
                })?;
            }
            Meta::NameValue(nv) => {
                if let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                {
                    out.desc = Some(s.value());
                } else {
                    return Err(Error::new(nv.span(), "expected a string description"));
                }
            }
            Meta::Path(_) => {
                return Err(Error::new(
                    attr.span(),
                    "expected #[s1(\"description\")] or #[s1(desc = \"...\")]",
                ));
            }
        }
    }
    Ok(out)
}

pub fn parse_field_attr(field: &Field) -> Result<FieldAttr> {
    let mut out = FieldAttr::default();
    for attr in &field.attrs {
        if !attr.path().is_ident("s1") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("noul") {
                let value = meta.value()?;
                let s: LitStr = value.parse()?;
                out.noul = Some(s.value());
                Ok(())
            } else if meta.path.is_ident("key") {
                let value = meta.value()?;
                let s: LitStr = value.parse()?;
                out.key = Some(s.value());
                Ok(())
            } else if meta.path.is_ident("true_means") {
                let value = meta.value()?;
                let s: LitStr = value.parse()?;
                out.true_means = Some(s.value());
                Ok(())
            } else if meta.path.is_ident("false_means") {
                let value = meta.value()?;
                let s: LitStr = value.parse()?;
                out.false_means = Some(s.value());
                Ok(())
            } else if meta.path.is_ident("flatten") {
                out.flatten = true;
                Ok(())
            } else {
                Err(meta.error("unrecognized s1 field attribute"))
            }
        })?;
    }
    Ok(out)
}

pub fn ident_or_err(name: &str, span: proc_macro2::Span) -> Result<Ident> {
    syn::parse_str::<Ident>(name)
        .map_err(|_| Error::new(span, format!("invalid identifier {name}")))
}
