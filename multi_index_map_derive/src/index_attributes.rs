use manyhow::bail;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    spanned::Spanned, DeriveInput, Field, Meta, MetaList, Path,
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
pub(crate) fn get_index_kind(f: &Field) -> syn::Result<Option<(Ordering, Uniqueness)>> {
    let mut ident_buf = String::new();
    for attr in &f.attrs {
        if attr.path().is_ident("multi_index") {
            let mut out = None;
            attr.parse_nested_meta(|meta| {
                out = match meta.path.get_ident().map(|i| { ident_buf = i.to_string(); &*ident_buf }) {
                    Some("hashed_unique") => Some((Ordering::Hashed, Uniqueness::Unique)),
                    Some("ordered_unique") => Some((Ordering::Ordered, Uniqueness::Unique)),
                    Some("hashed_non_unique") => Some((Ordering::Hashed, Uniqueness::NonUnique)),
                    Some("ordered_non_unique") => Some((Ordering::Ordered, Uniqueness::NonUnique)),
                    _ => {
                        bail!(meta.path.span(), "Invalid multi_index attribute, should be one of [hashed_unique, ordered_unique, hashed_non_unique, ordered_non_unique]");
                    }
                };
                Ok(())
            })?;
            return Ok(out);
        }
    }
    Ok(None)
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
            delimiter: syn::MacroDelimiter::Paren(syn::token::Paren(Span::call_site())),
            tokens: ident.into_token_stream(),
        });

        self.derives.push(derive);
    }
}

pub(crate) fn get_extra_attributes(f: &DeriveInput) -> syn::Result<ExtraAttributes> {
    let mut extra_attrs = ExtraAttributes::default();

    for attr in &f.attrs {
        if attr.path().is_ident("multi_index_derive") {
            attr.parse_nested_meta(|meta| {
                if let Some(ident) = meta.path.get_ident() {
                    extra_attrs.add_derive(ident);
                }
                Ok(())
            })?;
        }

        if attr.path().is_ident("multi_index_hash") {
            attr.parse_nested_meta(|meta| {
                extra_attrs.hasher = meta.path.clone();
                Ok(())
            })?;
        }
    }

    Ok(extra_attrs)
}
