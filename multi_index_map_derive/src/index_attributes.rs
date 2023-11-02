use ::syn::Field;
use proc_macro2::Span;
use proc_macro_error::emit_error;
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
    for attr in f.attrs.iter() {
        if attr.path.is_ident("multi_index") {
            let meta_list = match attr.parse_meta() {
                Ok(syn::Meta::List(l)) => l,
                _ => return None,
            };
            let nested = meta_list.nested.first()?;
            let nested_path = match nested {
                syn::NestedMeta::Meta(syn::Meta::Path(p)) => p,
                _ => return None,
            };

            if nested_path.is_ident("hashed_unique") {
                return Some((Ordering::Hashed, Uniqueness::Unique));
            } else if nested_path.is_ident("ordered_unique") {
                return Some((Ordering::Ordered, Uniqueness::Unique));
            } else if nested_path.is_ident("hashed_non_unique") {
                return Some((Ordering::Hashed, Uniqueness::NonUnique));
            } else if nested_path.is_ident("ordered_non_unique") {
                return Some((Ordering::Ordered, Uniqueness::NonUnique));
            } else {
                emit_error!(nested_path.span(), "Invalid multi_index attribute, should be one of [hashed_unique, ordered_unique, hashed_non_unique, ordered_non_unique]");
                return None;
            }
        }
    }
    None
}

#[derive(Default)]
pub(crate) struct ExtraAttributes {
    pub(crate) derives: Vec<Meta>,
}

impl ExtraAttributes {
    /// Add a single trait from `#[soa_derive]`
    fn add_derive(&mut self, ident: &proc_macro2::Ident) {
        let derive = Meta::List(MetaList {
            path: Path::from(syn::Ident::new("derive", Span::call_site())),
            paren_token: syn::token::Paren(Span::call_site()),
            nested: Punctuated::from_iter([NestedMeta::Meta(Meta::Path(Path::from(
                ident.clone(),
            )))]),
        });

        self.derives.push(derive)
    }
}

pub(crate) fn get_extra_attributes(f: &DeriveInput) -> ExtraAttributes {
    let mut extra_attrs = ExtraAttributes::default();

    for attr in f.attrs.iter() {
        if attr.path.is_ident("multi_index_derive") {
            let meta_list = match attr.parse_meta() {
                Ok(syn::Meta::List(l)) => l,
                _ => break,
            };
            for nested in meta_list.nested.iter() {
                let nested_path = match nested {
                    syn::NestedMeta::Meta(syn::Meta::Path(p)) => p,
                    _ => {
                        emit_error!(
                            nested.span(),
                            "Invalid multi_index_derive attribute, should be one of [Clone]"
                        );
                        continue;
                    }
                };

                let Some(ident) = nested_path.get_ident() else {
                    continue;
                };

                extra_attrs.add_derive(ident);
            }
        }
    }

    extra_attrs
}
