use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit_mut::{self, VisitMut};
use syn::{
    parse_quote, Data, DeriveInput, Error, Fields, Generics, Ident, Meta, Path, Type, Visibility,
};

#[derive(Clone, Debug)]
pub(crate) struct Field {
    pub(crate) ident: Ident,
    pub(crate) ty: Type,
    pub(crate) vis: Visibility,
}

#[derive(Clone, Debug)]
pub(crate) struct Index {
    pub(crate) source: IndexSource,
    pub(crate) fields: Vec<Field>,
    pub(crate) ordinal: usize,
}

impl Index {
    pub(crate) fn single_field(&self) -> Option<&Field> {
        (self.fields.len() == 1).then(|| &self.fields[0])
    }

    pub(crate) fn selector(&self) -> Option<&Path> {
        match &self.source {
            IndexSource::Selector(path) => Some(path),
            IndexSource::Legacy(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum IndexCategory {
    HashedUnique,
    HashedNonUnique,
    OrderedUnique,
    OrderedNonUnique,
}

#[derive(Clone, Debug)]
pub(crate) enum IndexSource {
    Legacy(IndexCategory),
    Selector(Path),
}

impl IndexSource {
    fn span(&self) -> proc_macro2::Span {
        match self {
            Self::Legacy(_) => proc_macro2::Span::call_site(),
            Self::Selector(path) => path.span(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Input {
    pub(crate) element: Ident,
    pub(crate) vis: Visibility,
    pub(crate) generics: Generics,
    pub(crate) indexes: Vec<Index>,
    pub(crate) unindexed: Vec<Field>,
}

impl Input {
    pub(crate) fn child_visibility(&self) -> Visibility {
        let mut vis = self.vis.clone();
        rebase_visibility(&mut vis);
        vis
    }

    pub(crate) fn rebase_for_child_module(&mut self) {
        let mut rebasing = RebaseForChildModule;
        rebasing.visit_generics_mut(&mut self.generics);
        for index in &mut self.indexes {
            if let IndexSource::Selector(selector) = &mut index.source {
                rebasing.visit_path_mut(selector);
            }
            for field in &mut index.fields {
                rebasing.visit_type_mut(&mut field.ty);
                rebase_visibility(&mut field.vis);
            }
        }
        for field in &mut self.unindexed {
            rebasing.visit_type_mut(&mut field.ty);
            rebase_visibility(&mut field.vis);
        }
    }
}

struct RebaseForChildModule;

impl VisitMut for RebaseForChildModule {
    fn visit_path_mut(&mut self, path: &mut Path) {
        if path.leading_colon.is_none() {
            if path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "self")
            {
                path.segments[0] = parent_segment();
            } else if path
                .segments
                .first()
                .is_some_and(|segment| segment.ident == "super")
            {
                path.segments.insert(0, parent_segment());
            }
        }
        visit_mut::visit_path_mut(self, path);
    }
}

fn parent_segment() -> syn::PathSegment {
    let path: Path = parse_quote!(super::placeholder);
    path.segments[0].clone()
}

fn rebase_visibility(vis: &mut Visibility) {
    match vis {
        Visibility::Inherited => *vis = parse_quote!(pub(super)),
        Visibility::Restricted(restricted) if restricted.path.is_ident("self") => {
            *vis = parse_quote!(pub(super));
        }
        Visibility::Restricted(restricted) if restricted.path.is_ident("super") => {
            *vis = parse_quote!(pub(in super::super));
        }
        Visibility::Restricted(restricted) => {
            RebaseForChildModule.visit_path_mut(&mut restricted.path);
        }
        Visibility::Public(_) => {}
    }
}

impl Input {
    pub(crate) fn parse(input: DeriveInput) -> syn::Result<Self> {
        let mut errors = None;

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

        let mut indexes = Vec::<Index>::new();
        let mut unindexed = Vec::new();
        if let Some(fields) = named {
            for field in fields {
                let value = Field {
                    ident: field.ident.clone().expect("named fields have identifiers"),
                    ty: field.ty.clone(),
                    vis: field.vis.clone(),
                };
                match parse_index_declaration(&field, &mut errors) {
                    None => unindexed.push(value),
                    Some(FieldIndexDeclaration::Legacy(category)) => {
                        indexes.push(Index {
                            source: IndexSource::Legacy(category),
                            fields: vec![value],
                            ordinal: indexes.len(),
                        });
                    }
                    Some(FieldIndexDeclaration::Selectors(selectors)) => {
                        for selector in selectors {
                            let key = path_key(&selector);
                            if let Some(index) = indexes.iter_mut().find(|index| {
                                index
                                    .selector()
                                    .is_some_and(|existing| path_key(existing) == key)
                            }) {
                                index.fields.push(value.clone());
                            } else {
                                indexes.push(Index {
                                    source: IndexSource::Selector(selector),
                                    fields: vec![value.clone()],
                                    ordinal: indexes.len(),
                                });
                            }
                        }
                    }
                }
            }
        }

        if indexes.is_empty() {
            push_error(
                &mut errors,
                Error::new(
                    input.ident.span(),
                    "MultiIndexMap2 requires at least one indexed field",
                ),
            );
        }
        for index in &indexes {
            if index.fields.len() > 12 {
                push_error(
                    &mut errors,
                    Error::new(
                        index.source.span(),
                        "compound indexes support at most 12 fields",
                    ),
                );
            }
        }

        if let Some(errors) = errors {
            return Err(errors);
        }

        Ok(Self {
            element: input.ident,
            vis: input.vis,
            generics: input.generics,
            indexes,
            unindexed,
        })
    }
}

enum FieldIndexDeclaration {
    Legacy(IndexCategory),
    Selectors(Vec<Path>),
}

fn parse_index_declaration(
    field: &syn::Field,
    errors: &mut Option<Error>,
) -> Option<FieldIndexDeclaration> {
    let attrs = field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("multi_index"))
        .collect::<Vec<_>>();
    for attr in attrs.iter().skip(1) {
        push_error(
            errors,
            Error::new(
                attr.span(),
                "use one #[multi_index(...)] attribute per field",
            ),
        );
    }
    let attr = attrs.first()?;
    let metas = match attr
        .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
    {
        Ok(metas) => metas,
        Err(error) => {
            push_error(errors, error);
            return None;
        }
    };
    if metas.is_empty() {
        push_error(
            errors,
            Error::new(
                attr.span(),
                "#[multi_index(...)] requires one category or at least one by(Selector)",
            ),
        );
        return None;
    }

    let mut category = None;
    let mut selectors = Vec::new();
    for meta in metas {
        match meta {
            Meta::Path(path) => {
                let Some(parsed) = parse_category(&path) else {
                    push_error(
                        errors,
                        Error::new(
                            path.span(),
                            "bare selector paths are unsupported; use by(Selector)",
                        ),
                    );
                    continue;
                };
                if category.replace(parsed).is_some() {
                    push_error(
                        errors,
                        Error::new(path.span(), "only one legacy index category is allowed"),
                    );
                }
            }
            Meta::List(list) if list.path.is_ident("by") => {
                let selector = match list.parse_args::<Path>() {
                    Ok(selector) => selector,
                    Err(error) => {
                        push_error(errors, error);
                        continue;
                    }
                };
                let key = path_key(&selector);
                if selectors.iter().any(|existing| path_key(existing) == key) {
                    push_error(
                        errors,
                        Error::new(
                            selector.span(),
                            "duplicate use of this selector on the same field",
                        ),
                    );
                    continue;
                }
                selectors.push(selector);
            }
            other => push_error(
                errors,
                Error::new(
                    other.span(),
                    "expected a legacy index category or by(Selector)",
                ),
            ),
        }
    }

    if category.is_some() && !selectors.is_empty() {
        push_error(
            errors,
            Error::new(
                attr.span(),
                "a field cannot mix a legacy index category with by(Selector)",
            ),
        );
        return None;
    }
    match (category, selectors.is_empty()) {
        (Some(category), true) => Some(FieldIndexDeclaration::Legacy(category)),
        (None, false) => Some(FieldIndexDeclaration::Selectors(selectors)),
        _ => None,
    }
}

fn parse_category(path: &Path) -> Option<IndexCategory> {
    match path.get_ident()?.to_string().as_str() {
        "hashed_unique" => Some(IndexCategory::HashedUnique),
        "hashed_non_unique" => Some(IndexCategory::HashedNonUnique),
        "ordered_unique" => Some(IndexCategory::OrderedUnique),
        "ordered_non_unique" => Some(IndexCategory::OrderedNonUnique),
        _ => None,
    }
}

fn path_key(path: &Path) -> String {
    path.to_token_stream().to_string().replace(' ', "")
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
    fn groups_overlapping_compound_indexes() {
        let input: DeriveInput = parse_quote! {
            struct Order {
                #[multi_index(by(ById))]
                id: u64,
                #[multi_index(by(crate::ByTraderTimestamp))]
                trader: String,
                #[multi_index(by(ByTimestamp), by(crate::ByTraderTimestamp))]
                timestamp: u64,
                note: String,
            }
        };
        let parsed = Input::parse(input).unwrap();
        assert_eq!(parsed.indexes.len(), 3);
        assert_eq!(parsed.indexes[1].fields.len(), 2);
        assert_eq!(parsed.indexes[1].fields[0].ident, "trader");
        assert_eq!(parsed.indexes[1].fields[1].ident, "timestamp");
        assert_eq!(parsed.unindexed.len(), 1);
    }

    #[test]
    fn rejects_duplicate_selector_on_one_field() {
        let input: DeriveInput = parse_quote! {
            struct Bad {
                #[multi_index(by(ById), by(ById))]
                id: u64,
            }
        };
        assert!(Input::parse(input).is_err());
    }

    #[test]
    fn rejects_multiple_index_attributes_on_one_field() {
        let input: DeriveInput = parse_quote! {
            struct Bad {
                #[multi_index(by(ById))]
                #[multi_index(by(ByTimestamp))]
                id: u64,
            }
        };
        assert!(Input::parse(input).is_err());
    }

    #[test]
    fn accepts_legacy_index_kind_syntax() {
        let input: DeriveInput = parse_quote! {
            struct Record {
                #[multi_index(hashed_unique)]
                id: u64,
                #[multi_index(ordered_non_unique)]
                rank: u64,
            }
        };
        let parsed = Input::parse(input).unwrap();
        assert!(matches!(
            parsed.indexes[0].source,
            IndexSource::Legacy(IndexCategory::HashedUnique)
        ));
        assert!(matches!(
            parsed.indexes[1].source,
            IndexSource::Legacy(IndexCategory::OrderedNonUnique)
        ));
    }

    #[test]
    fn rejects_bare_selectors_mixed_forms_and_multiple_categories() {
        for input in [
            parse_quote! {
                struct Bad {
                    #[multi_index(ById)]
                    id: u64,
                }
            },
            parse_quote! {
                struct Bad {
                    #[multi_index(hashed_unique, by(ById))]
                    id: u64,
                }
            },
            parse_quote! {
                struct Bad {
                    #[multi_index(hashed_unique, ordered_unique)]
                    id: u64,
                }
            },
        ] {
            assert!(Input::parse(input).is_err());
        }
    }

    #[test]
    fn rejects_malformed_by_and_empty_attributes() {
        for input in [
            parse_quote! {
                struct Bad {
                    #[multi_index(by())]
                    id: u64,
                }
            },
            parse_quote! {
                struct Bad {
                    #[multi_index(by(ById, ByOther))]
                    id: u64,
                }
            },
            parse_quote! {
                struct Bad {
                    #[multi_index()]
                    id: u64,
                }
            },
        ] {
            assert!(Input::parse(input).is_err());
        }
    }

    #[test]
    fn category_name_inside_by_is_a_selector() {
        let input: DeriveInput = parse_quote! {
            struct Record {
                #[multi_index(by(hashed_non_unique))]
                id: u64,
            }
        };
        let parsed = Input::parse(input).unwrap();
        assert_eq!(
            path_key(parsed.indexes[0].selector().unwrap()),
            "hashed_non_unique"
        );
    }

    #[test]
    fn rebases_child_module_paths_macros_and_visibilities() {
        let input: DeriveInput = parse_quote! {
            struct Record {
                #[multi_index(by(self::ById))]
                pub(self) id: self::key_type!(),
                pub(super) payload: super::Payload,
                pub(in crate::scope) scoped: u8,
            }
        };
        let mut parsed = Input::parse(input).unwrap();
        assert_eq!(
            parsed.child_visibility().to_token_stream().to_string(),
            "pub (super)"
        );

        parsed.rebase_for_child_module();
        assert_eq!(
            path_key(parsed.indexes[0].selector().unwrap()),
            "super::ById"
        );
        assert_eq!(
            parsed.indexes[0].fields[0]
                .ty
                .to_token_stream()
                .to_string()
                .replace(' ', ""),
            "super::key_type!()"
        );
        assert_eq!(
            parsed.unindexed[0]
                .ty
                .to_token_stream()
                .to_string()
                .replace(' ', ""),
            "super::super::Payload"
        );
        assert_eq!(
            parsed.indexes[0].fields[0]
                .vis
                .to_token_stream()
                .to_string(),
            "pub (super)"
        );
        assert_eq!(
            parsed.unindexed[0].vis.to_token_stream().to_string(),
            "pub (in super :: super)"
        );
        assert_eq!(
            parsed.unindexed[1].vis.to_token_stream().to_string(),
            "pub (in crate :: scope)"
        );
    }

    #[test]
    fn preserves_and_rebases_generic_parameters() {
        let input: DeriveInput = parse_quote! {
            struct Record<
                'a,
                T: self::Bound = super::DefaultType,
                const N: usize = { self::DEFAULT_SIZE },
            >
            where
                T: crate::RootBound,
                [u8; N]: super::ArrayBound,
            {
                #[multi_index(by(ById))]
                id: u64,
                value: &'a T,
            }
        };
        let mut parsed = Input::parse(input).unwrap();
        parsed.rebase_for_child_module();

        let generics = parsed
            .generics
            .to_token_stream()
            .to_string()
            .replace(' ', "");
        assert!(generics.contains("T:super::Bound=super::super::DefaultType"));
        assert!(generics.contains("constN:usize={super::DEFAULT_SIZE}"));
        let where_clause = parsed
            .generics
            .where_clause
            .to_token_stream()
            .to_string()
            .replace(' ', "");
        assert!(where_clause.contains("T:crate::RootBound"));
        assert!(where_clause.contains("[u8;N]:super::super::ArrayBound"));
    }
}
