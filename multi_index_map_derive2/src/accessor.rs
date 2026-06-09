use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Fields};

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return Err(Error::new(
            input.generics.span(),
            "MultiIndexAccessor does not support generic structs",
        ));
    }

    match &input.data {
        Data::Struct(data) if matches!(data.fields, Fields::Unit) => {}
        Data::Struct(data) => {
            return Err(Error::new(
                data.fields.span(),
                "MultiIndexAccessor requires a unit struct",
            ));
        }
        Data::Enum(data) => {
            return Err(Error::new(
                data.enum_token.span,
                "MultiIndexAccessor can only be derived for unit structs",
            ));
        }
        Data::Union(data) => {
            return Err(Error::new(
                data.union_token.span,
                "MultiIndexAccessor can only be derived for unit structs",
            ));
        }
    }

    let attrs = input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("multi_index"))
        .collect::<Vec<_>>();
    if attrs.len() != 1 {
        return Err(Error::new(
            input.ident.span(),
            "MultiIndexAccessor requires exactly one #[multi_index(...)] category",
        ));
    }

    let attr = attrs[0];
    let paths = attr.parse_args_with(
        syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
    )?;
    if paths.len() != 1 {
        return Err(Error::new(
            attr.span(),
            "#[multi_index(...)] requires exactly one index category",
        ));
    }
    let path = paths.first().expect("checked one path");
    let Some(category) = path.get_ident() else {
        return Err(Error::new(
            path.span(),
            "index category must be a single identifier",
        ));
    };
    let kind = match category.to_string().as_str() {
        "hashed_unique" => quote!(::multi_index_map::__private::HashedUnique),
        "hashed_non_unique" => quote!(::multi_index_map::__private::HashedNonUnique),
        "ordered_unique" => quote!(::multi_index_map::__private::OrderedUnique),
        "ordered_non_unique" => quote!(::multi_index_map::__private::OrderedNonUnique),
        _ => {
            return Err(Error::new(
                category.span(),
                "invalid index category; expected hashed_unique, hashed_non_unique, ordered_unique, or ordered_non_unique",
            ));
        }
    };

    let ident = input.ident;
    Ok(quote! {
        impl ::multi_index_map::MultiIndexAccessor for #ident {
            type Kind = #kind;
            const NAME: &'static str = stringify!(#ident);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn accepts_each_category() {
        for category in [
            "hashed_unique",
            "hashed_non_unique",
            "ordered_unique",
            "ordered_non_unique",
        ] {
            let category: syn::Ident = syn::parse_str(category).unwrap();
            let input: DeriveInput = parse_quote! {
                #[multi_index(#category)]
                struct Accessor;
            };
            assert!(generate(input).is_ok());
        }
    }

    #[test]
    fn rejects_non_unit_generic_and_missing_categories() {
        let non_unit: DeriveInput = parse_quote! {
            #[multi_index(hashed_unique)]
            struct Accessor(u8);
        };
        assert!(generate(non_unit).is_err());

        let generic: DeriveInput = parse_quote! {
            #[multi_index(hashed_unique)]
            struct Accessor<T>;
        };
        assert!(generate(generic).is_err());

        let missing: DeriveInput = parse_quote! { struct Accessor; };
        assert!(generate(missing).is_err());
    }
}
