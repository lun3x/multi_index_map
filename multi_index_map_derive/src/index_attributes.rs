use ::syn::Field;
use proc_macro2::Span;
use proc_macro_error2::emit_error;
use syn::{
    punctuated::Punctuated, spanned::Spanned, DeriveInput, Meta, MetaList, NestedMeta, Path,
};

// Represents whether the index is Ordered or Hashed, ie. whether we use a BTreeMap or a FxHashMap
//   as the lookup table.
pub(crate) enum Ordering {
    Hashed,
    Ordered,
}

// Represents whether the index is Unique or NonUnique, ie. whether we allow multiple elements with the same
//   value in this index.
// All these variants end in Unique, even "NonUnique", remove this warning.
#[allow(clippy::enum_variant_names)]
pub(crate) enum Uniqueness {
    Unique,
    NonUnique,
}

// Get the Ordering and Uniqueness for a given field attribute.
pub(crate) fn get_index_kind(f: &Field) -> Option<(Ordering, Uniqueness)> {
    let mut ident_buf = String::new();
    for attr in &f.attrs {
        if attr.path.is_ident("multi_index") {
            return {
                let Ok(syn::Meta::List(meta_list)) = attr.parse_meta() else { return None };
                let nested = meta_list.nested.first()?;
                let syn::NestedMeta::Meta(syn::Meta::Path(nested_path)) = nested else { return None };

                match nested_path.get_ident().map(|i| { ident_buf = i.to_string(); &*ident_buf }) {
                    Some("hashed_unique") => Some((Ordering::Hashed, Uniqueness::Unique)),
                    Some("ordered_unique") => Some((Ordering::Ordered, Uniqueness::Unique)),
                    Some("hashed_non_unique") => Some((Ordering::Hashed, Uniqueness::NonUnique)),
                    Some("ordered_non_unique") => Some((Ordering::Ordered, Uniqueness::NonUnique)),
                    _ => {
                        emit_error!(nested_path.span(), "Invalid multi_index attribute, should be one of [hashed_unique, ordered_unique, hashed_non_unique, ordered_non_unique]");
                        None
                    }
                }
            }
        }
    }
    None
}

pub(crate) struct ExtraAttributes {
    pub(crate) derives: Vec<Meta>,
    pub(crate) hasher: syn::Path,
}

impl Default for ExtraAttributes {
    fn default() -> Self {
        Self {
            derives: Default::default(),
            #[cfg(feature = "rustc-hash")]
            hasher: syn::parse_quote!(::multi_index_map::rustc_hash::FxBuildHasher),
            #[cfg(not(feature = "rustc-hash"))]
            hasher: syn::parse_quote!(::std::hash::RandomState),
        }
    }
}

impl ExtraAttributes {
    /// Add a single trait from `#[multi_index_derive]`
    fn add_derive(&mut self, ident: &proc_macro2::Ident) {
        // We hardcode derive(Default) because this is always possible, so no need to explicitly add it here
        if ident == "Default" {
            return;
        }

        let derive = Meta::List(MetaList {
            path: Path::from(syn::Ident::new("derive", Span::call_site())),
            paren_token: syn::token::Paren(Span::call_site()),
            nested: Punctuated::from_iter([NestedMeta::Meta(Meta::Path(Path::from(
                ident.clone(),
            )))]),
        });

        self.derives.push(derive);
    }
}

pub(crate) fn get_extra_attributes(f: &DeriveInput) -> ExtraAttributes {
    let mut extra_attrs = ExtraAttributes::default();

    for attr in &f.attrs {
        if attr.path.is_ident("multi_index_derive") {
            let Ok(syn::Meta::List(meta_list)) = attr.parse_meta() else {
                break
            };
            for nested in &meta_list.nested {
                let syn::NestedMeta::Meta(syn::Meta::Path(nested_path)) = nested else {
                    emit_error!(
                        nested.span(),
                        "Invalid multi_index_derive attribute, should be a deriveable trait, eg. Clone, Debug"
                    );
                    continue;
                };

                let Some(ident) = nested_path.get_ident() else {
                    continue;
                };

                extra_attrs.add_derive(ident);
            }
        }

        if attr.path.is_ident("multi_index_hash") {
            let Ok(syn::Meta::List(meta_list)) = attr.parse_meta() else {
                break
            };
            for nested in &meta_list.nested {
                let syn::NestedMeta::Meta(syn::Meta::Path(nested_path)) = nested else {
                    emit_error!(
                        nested.span(),
                        "Invalid multi_index_hash attribute, should be a struct implementing BuildHasher eg. FxBuildHasher"
                    );
                    continue;
                };

                extra_attrs.hasher = nested_path.clone();
                break;
            }
        }
    }

    extra_attrs
}
