use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Fields, Ident, Type, Visibility};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ordering {
    Hashed,
    Ordered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Uniqueness {
    Unique,
    NonUnique,
}

#[derive(Clone, Debug)]
pub(crate) struct IndexedField {
    pub(crate) ident: Ident,
    pub(crate) ty: Type,
    pub(crate) vis: Visibility,
    pub(crate) ordering: Ordering,
    pub(crate) uniqueness: Uniqueness,
}

#[derive(Clone, Debug)]
pub(crate) struct UnindexedField {
    pub(crate) ident: Ident,
    pub(crate) ty: Type,
    pub(crate) vis: Visibility,
}

#[derive(Debug)]
pub(crate) struct Input {
    pub(crate) element: Ident,
    pub(crate) vis: Visibility,
    pub(crate) indexed: Vec<IndexedField>,
    pub(crate) unindexed: Vec<UnindexedField>,
}

impl Input {
    pub(crate) fn parse(input: DeriveInput) -> syn::Result<Self> {
        let mut errors = None;

        if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
            push_error(
                &mut errors,
                Error::new(
                    input.generics.span(),
                    "MultiIndexMap2 does not support generic structs yet",
                ),
            );
        }

        let named = match input.data {
            Data::Struct(data) => match data.fields {
                Fields::Named(fields) => Some(fields.named),
                other => {
                    push_error(
                        &mut errors,
                        Error::new(
                            other.span(),
                            "MultiIndexMap2 requires a struct with named fields",
                        ),
                    );
                    None
                }
            },
            Data::Enum(data) => {
                push_error(
                    &mut errors,
                    Error::new(
                        data.enum_token.span,
                        "MultiIndexMap2 can only be derived for structs",
                    ),
                );
                None
            }
            Data::Union(data) => {
                push_error(
                    &mut errors,
                    Error::new(
                        data.union_token.span,
                        "MultiIndexMap2 can only be derived for structs",
                    ),
                );
                None
            }
        };

        let mut indexed = Vec::new();
        let mut unindexed = Vec::new();
        if let Some(fields) = named {
            for field in fields {
                let ident = field.ident.clone().expect("named fields have identifiers");
                match parse_index_kind(&field, &mut errors) {
                    Some((ordering, uniqueness)) => indexed.push(IndexedField {
                        ident,
                        ty: field.ty,
                        vis: field.vis,
                        ordering,
                        uniqueness,
                    }),
                    None => unindexed.push(UnindexedField {
                        ident,
                        ty: field.ty,
                        vis: field.vis,
                    }),
                }
            }
        }

        if indexed.is_empty() {
            push_error(
                &mut errors,
                Error::new(
                    input.ident.span(),
                    "MultiIndexMap2 requires at least one #[multi_index(...)] field",
                ),
            );
        }

        if let Some(errors) = errors {
            return Err(errors);
        }

        Ok(Self {
            element: input.ident,
            vis: input.vis,
            indexed,
            unindexed,
        })
    }
}

fn parse_index_kind(
    field: &syn::Field,
    errors: &mut Option<Error>,
) -> Option<(Ordering, Uniqueness)> {
    let attrs = field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("multi_index"))
        .collect::<Vec<_>>();

    if attrs.len() > 1 {
        for attr in attrs.iter().skip(1) {
            push_error(
                errors,
                Error::new(attr.span(), "duplicate #[multi_index(...)] attribute"),
            );
        }
    }
    let attr = attrs.first()?;

    let paths = match attr
        .parse_args_with(syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
    {
        Ok(paths) => paths,
        Err(error) => {
            push_error(errors, error);
            return None;
        }
    };

    if paths.len() != 1 {
        push_error(
            errors,
            Error::new(
                attr.span(),
                "#[multi_index(...)] requires exactly one index kind",
            ),
        );
        return None;
    }

    let path = paths.first().expect("checked one path");
    let Some(ident) = path.get_ident() else {
        push_error(
            errors,
            Error::new(path.span(), "index kind must be a single identifier"),
        );
        return None;
    };

    match ident.to_string().as_str() {
        "hashed_unique" => Some((Ordering::Hashed, Uniqueness::Unique)),
        "hashed_non_unique" => Some((Ordering::Hashed, Uniqueness::NonUnique)),
        "ordered_unique" => Some((Ordering::Ordered, Uniqueness::Unique)),
        "ordered_non_unique" => Some((Ordering::Ordered, Uniqueness::NonUnique)),
        _ => {
            push_error(
                errors,
                Error::new(
                    ident.span(),
                    "invalid index kind; expected hashed_unique, hashed_non_unique, ordered_unique, or ordered_non_unique",
                ),
            );
            None
        }
    }
}

fn push_error(errors: &mut Option<Error>, error: Error) {
    if let Some(errors) = errors {
        errors.combine(error);
    } else {
        *errors = Some(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn parses_all_index_kinds() {
        let input: DeriveInput = parse_quote! {
            struct Order {
                #[multi_index(hashed_unique)]
                id: u64,
                #[multi_index(hashed_non_unique)]
                trader: String,
                #[multi_index(ordered_unique)]
                timestamp: u64,
                #[multi_index(ordered_non_unique)]
                price: u64,
                note: String,
            }
        };
        let parsed = Input::parse(input).unwrap();
        assert_eq!(parsed.indexed.len(), 4);
        assert_eq!(parsed.unindexed.len(), 1);
    }

    #[test]
    fn rejects_generics_and_missing_indexes_together() {
        let input: DeriveInput = parse_quote! {
            struct Generic<T> {
                value: T,
            }
        };
        let error = Input::parse(input).unwrap_err().to_string();
        assert!(error.contains("generic structs"));
    }

    #[test]
    fn rejects_malformed_attributes() {
        let input: DeriveInput = parse_quote! {
            struct Bad {
                #[multi_index(hashed_unique, ordered_unique)]
                value: u64,
            }
        };
        assert!(Input::parse(input).is_err());
    }
}
