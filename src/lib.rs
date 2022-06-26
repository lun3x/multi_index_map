use proc_macro::TokenStream;
use proc_macro_error::{abort_call_site, proc_macro_error};
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(MultiIndexMap, attributes(multi_index))]
#[proc_macro_error]
pub fn multi_index_map(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Extract the struct fields if we are parsing a struct, otherwise throw an error as we do not support Enums or Unions
    let fields = match input.data {
        syn::Data::Struct(d) => d.fields,
        _ => {
            abort_call_site!("MultiIndexMap only support structs as elements")
        }
    };

    // For each field generate a TokenStream representing the mapped index to the main store
    let lookup_table_fields: Vec<quote::__private::TokenStream> =
        if let syn::Fields::Named(f) = &fields {
            f.named
                .iter()
                .map(|f| {
                    let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
                    let ty = &f.ty;
                    if let Some(attr) = f.attrs.first() {
                        if attr.path.is_ident("multi_index") {
                            quote! {
                                #index_name: rustc_hash::FxHashMap<#ty, usize>,
                            }
                        } else {
                            quote! {}
                        }
                    } else {
                        quote! {}
                    }
                })
                .collect()
        } else {
            abort_call_site!(
                "MultiIndexMap only support named struct fields, not unnamed tuple structs"
            )
        };

    let element_name = input.ident;

    // For each field generate a TokenStream representing the remove from that field's lookup table
    let removes: Vec<quote::__private::TokenStream> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| {
                let field_name = f.ident.as_ref().unwrap();
                let index_name = format_ident!("_{}_index", field_name);

                if let Some(attr) = f.attrs.first() {
                    if attr.path.is_ident("multi_index") {
                        quote! {
                            self.#index_name.remove(&elem.#field_name);
                        }
                    } else {
                        quote! {}
                    }
                } else {
                    quote! {}
                }
            })
            .collect()
    } else {
        abort_call_site!(
            "MultiIndexMap only support named struct fields, not unnamed tuple structs"
        )
    };

    // For each field generate a TokenStream representing the remover for the underlying storage via that field's lookup table
    let removers: Vec<quote::__private::TokenStream> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| {
                let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
                let remover_name = format_ident!("remove_by_{}", f.ident.as_ref().unwrap());
                let ty = &f.ty;

                if let Some(attr) = f.attrs.first() {
                    if attr.path.is_ident("multi_index") {
                        quote! {
                            pub(super) fn #remover_name(&mut self, key: &#ty) -> Option<#element_name> {
                                let idx = self.#index_name.remove(key)?;
                                let elem = self._store.remove(idx);
                                #(#removes)*
                                Some(elem)
                            }
                        }
                    } else {
                        quote! {}
                    }
                } else {
                    quote! {}
                }
            })
            .collect()
    } else {
        abort_call_site!(
            "MultiIndexMap only support named struct fields, not unnamed tuple structs"
        )
    };

    // For each field generate a TokenStream representing the accessor for the underlying storage via that field's lookup table
    let accessors: Vec<quote::__private::TokenStream> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| {
                let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
                let accessor_name = format_ident!("get_by_{}", f.ident.as_ref().unwrap());
                let mut_accessor_name = format_ident!("get_mut_by_{}", f.ident.as_ref().unwrap());
                let ty = &f.ty;

                if let Some(attr) = f.attrs.first() {
                    if attr.path.is_ident("multi_index") {
                        quote! {
                            pub(super) fn #accessor_name(&self, key: &#ty) -> Option<&#element_name> {
                                self._store.get(*self.#index_name.get(key)?)
                            }
        
                            pub(super) fn #mut_accessor_name(&mut self, key: &#ty) -> Option<&mut #element_name> {
                                self._store.get_mut(*self.#index_name.get(key)?)
                            }
        
                        }
                    } else {
                        quote! {}
                    }
                } else {
                    quote! {}
                }


            })
            .collect()
    } else {
        abort_call_site!(
            "MultiIndexMap only support named struct fields, not unnamed tuple structs"
        )
    };

    // For each field generate a TokenStream representing the insert to that field's lookup table
    let inserts: Vec<quote::__private::TokenStream> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| {
                let field_name = f.ident.as_ref().unwrap();
                let index_name = format_ident!("_{}_index", field_name);

                if let Some(attr) = f.attrs.first() {
                    if attr.path.is_ident("multi_index") {
                        quote! {
                            self.#index_name.insert(elem.#field_name, idx);
                        }
                    } else {
                        quote! {}
                    }
                } else {
                    quote! {}
                }
            })
            .collect()
    } else {
        abort_call_site!(
            "MultiIndexMap only support named struct fields, not unnamed tuple structs"
        )
    };

    // Generate the name of the MultiIndexMap
    let map_name = format_ident!("MultiIndex{}Map", element_name);

    let expanded = quote! {
        mod multi_index {
            use super::#element_name;

            #[derive(Debug, Default)]
            pub(super) struct #map_name {
                _store: slab::Slab<#element_name>,
                #(#lookup_table_fields)*
            }

            impl #map_name {
                pub(super) fn insert(&mut self, elem: #element_name) {
                    let idx = self._store.insert(elem);
                    let elem = &self._store[idx];

                    #(#inserts)*
                }

                #(#accessors)*

                #(#removers)*
            }
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
