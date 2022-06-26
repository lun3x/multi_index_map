use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(MultiIndexMap)]
pub fn multi_index_map(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Extract the struct fields if we are parsing a struct, otherwise throw an error as we do not support Enums or Unions
    let fields = match input.data {
        syn::Data::Struct(d) => d.fields,
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };

    // For each field generate a TokenStream representing the mapped index to the main store
    let tokens: Vec<quote::__private::TokenStream> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| {
                let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
                let ty = &f.ty;

                quote! {
                    #index_name: FxHashMap<#ty, u64>
                }
            })
            .collect()
    } else {
        todo!()
    };

    let name = input.ident;

    // For each field generate a TokenStream representing the accessor for the index
    let accessors: Vec<quote::__private::TokenStream> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| {
                let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
                let accessor_name = format_ident!("get_by_{}", f.ident.as_ref().unwrap());
                let ty = &f.ty;

                quote! {
                    fn #accessor_name(&self, key: &#ty) -> Option<&#name> {
                        self._store.get(self.#index_name.get(key)?)
                    }

                }
            })
            .collect()
    } else {
        todo!()
    };

    // Generate the name of the MultiIndexMap
    let map_name = format_ident!("MultiIndex{}Map", name);

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        #[derive(Debug, Default)]
        struct #map_name {
            _store: FxHashMap<u64, #name>,
            #(#tokens),*
        }

        impl #map_name {
            #(#accessors)*
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
