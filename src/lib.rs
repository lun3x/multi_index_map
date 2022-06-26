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
                    #index_name: FxHashMap<#ty, usize>
                }
            })
            .collect()
    } else {
        todo!()
    };

    let element_name = input.ident;

    // For each field generate a TokenStream representing the remove from that field's lookup table
    let removes: Vec<quote::__private::TokenStream> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| {
                let field_name = f.ident.as_ref().unwrap();
                let index_name = format_ident!("_{}_index", field_name);

                quote! {
                    self.#index_name.remove(&elem.#field_name);
                }
            })
            .collect()
    } else {
        todo!()
    };

    // For each field generate a TokenStream representing the remover for the underlying storage via that field's lookup table
    let removers: Vec<quote::__private::TokenStream> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| {
                let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
                let accessor_name = format_ident!("remove_by_{}", f.ident.as_ref().unwrap());
                let ty = &f.ty;

                quote! {
                    fn #accessor_name(&mut self, key: &#ty) -> Option<#element_name> {
                        let idx = self.#index_name.remove(key)?;
                        let elem = self._store.remove(idx);
                        #(#removes)*
                        Some(elem)
                    }

                }
            })
            .collect()
    } else {
        todo!()
    };

    // For each field generate a TokenStream representing the accessor for the underlying storage via that field's lookup table
    let accessors: Vec<quote::__private::TokenStream> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| {
                let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
                let accessor_name = format_ident!("get_by_{}", f.ident.as_ref().unwrap());
                let ty = &f.ty;

                quote! {
                    fn #accessor_name(&self, key: &#ty) -> Option<&#element_name> {
                        self._store.get(*self.#index_name.get(key)?)
                    }

                }
            })
            .collect()
    } else {
        todo!()
    };

    // For each field generate a TokenStream representing the insert to that field's lookup table
    let inserts: Vec<quote::__private::TokenStream> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| {
                let field_name = f.ident.as_ref().unwrap();
                let index_name = format_ident!("_{}_index", field_name);

                quote! {
                    self.#index_name.insert(elem.#field_name, idx);
                }
            })
            .collect()
    } else {
        todo!()
    };

    // Generate the name of the MultiIndexMap
    let map_name = format_ident!("MultiIndex{}Map", element_name);

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        #[derive(Debug, Default)]
        struct #map_name {
            _store: Vec<#element_name>,
            #(#tokens),*
        }

        impl #map_name {
            fn insert(&mut self, elem: #element_name) {
                let idx = self._store.len();

                #(#inserts)*

                self._store.push(elem);
            }

            #(#accessors)*

            #(#removers)*
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
