use convert_case::Casing;
use proc_macro_error::{abort_call_site, proc_macro_error};
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(MultiIndexMap, attributes(multi_index))]
#[proc_macro_error]
pub fn multi_index_map(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Extract the struct fields if we are parsing a struct, otherwise throw an error as we do not support Enums or Unions
    let fields = match input.data {
        syn::Data::Struct(d) => d.fields,
        _ => abort_call_site!("MultiIndexMap only supports structs as elements"),
    };

    let named_fields = match fields {
        syn::Fields::Named(f) => f,
        _ => abort_call_site!(
            "Struct fields must be named, unnamed tuple structs and unit structs are not supported"
        ),
    };

    let fields_to_index = || {
        named_fields.named.iter().filter(|f| {
            f.attrs.first().is_some() && f.attrs.first().unwrap().path.is_ident("multi_index")
        })
    };

    // For each indexed field generate a TokenStream representing the lookup table for that field
    let lookup_table_fields = fields_to_index().map(|f| {
        let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
        let ty = &f.ty;

        let index_kind = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });

        match index_kind {
            IndexKind::HashedUnique => {
                quote! {
                    #index_name: rustc_hash::FxHashMap<#ty, usize>,
                }
            }
            IndexKind::OrderedUnique => {
                quote! {
                    #index_name: std::collections::BTreeMap<#ty, usize>,
                }
            }
            IndexKind::HashedNonUnique => {
                quote! {
                    #index_name: rustc_hash::FxHashMap<#ty, Vec<usize>>,
                }
            }
        }
    });

    // For each indexed field generate a TokenStream representing the insert to that field's lookup table
    let inserts: Vec<proc_macro2::TokenStream> = fields_to_index()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let index_name = format_ident!("_{}_index", field_name);
            let index_kind = get_index_kind(f).unwrap_or_else(|| {
                abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
            });

            match index_kind {
                IndexKind::HashedNonUnique => {
                    quote! {
                        self.#index_name.entry(elem.#field_name.clone()).or_insert(Vec::with_capacity(1)).push(idx); 
                    }
                }
                _ => quote! { 
                    self.#index_name.insert(elem.#field_name.clone(), idx);
                },
            }
        })
        .collect();

    // For each indexed field generate a TokenStream representing the remove from that field's lookup table
    let removes: Vec<proc_macro2::TokenStream> = fields_to_index()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let index_name = format_ident!("_{}_index", field_name);
            quote! {
                self.#index_name.remove(&elem_orig.#field_name);
            }
        })
        .collect();


    // For each indexed field generate a TokenStream representing the combined remove and insert from that field's lookup table
    let modifies: Vec<proc_macro2::TokenStream> = fields_to_index().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let index_name = format_ident!("_{}_index", field_name);
        let index_kind = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });

        match index_kind {
            IndexKind::HashedNonUnique => {
                quote! {
                    let idxs = self.#index_name.remove(&elem_orig.#field_name).expect("Internal invariants broken, unable to find element in one index despite being present in other");
                    self.#index_name.entry(elem.#field_name.clone()).or_insert(Vec::with_capacity(1)).extend(idxs); 
                }
            }
            _ => quote! {
                let idx = self.#index_name.remove(&elem_orig.#field_name).expect("Internal invariants broken, unable to find element in one index despite being present in other");
                self.#index_name.insert(elem.#field_name.clone(), idx);
            }
        }

    }).collect();

    let element_name = input.ident;

    // Generate the name of the MultiIndexMap
    let map_name = format_ident!("MultiIndex{}Map", element_name);

    // For each indexed field generate a TokenStream representing an accessor for the underlying storage via that field's lookup table
    let accessors = fields_to_index().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let index_name = format_ident!("_{}_index", field_name);
        let getter_name = format_ident!("get_by_{}", field_name);
        let mut_getter_name = format_ident!("get_mut_by_{}", field_name);
        let remover_name = format_ident!("remove_by_{}", field_name);
        let modifier_name = format_ident!("modify_by_{}", field_name);
        let iter_name = format_ident!("{}{}Iter", map_name, field_name.to_string().to_case(convert_case::Case::UpperCamel));
        let iter_getter_name = format_ident!("iter_by_{}", field_name);
        let ty = &f.ty;
        let index_kind = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });

        let getter = match index_kind {
            IndexKind::HashedNonUnique => quote!{
                pub(super) fn #getter_name(&self, key: &#ty) -> Vec<&#element_name> {
                    if let Some(idxs) = self.#index_name.get(key) {
                        let mut elem_refs = Vec::with_capacity(idxs.len());
                        for idx in idxs {
                            elem_refs.push(&self._store[*idx])
                        }
                        elem_refs
                    } else {
                        Vec::new()
                    }
                }
            },
            _ => quote!{
                pub(super) fn #getter_name(&self, key: &#ty) -> Option<&#element_name> {
                    Some(&self._store[*self.#index_name.get(key)?])
                }
            }
        };

        let mut_getter = match index_kind {
            IndexKind::HashedNonUnique => quote!{},
            _ => quote!{
                // SAFETY:
                // It is safe to mutate the non-indexed fields, however mutating any of the indexed fields will break the internal invariants.
                // If the indexed fields need to be changed, the modify() method must be used.
                pub(super) unsafe fn #mut_getter_name(&mut self, key: &#ty) -> Option<&mut #element_name> {
                    Some(&mut self._store[*self.#index_name.get(key)?])
                }
            }
        };

        let remover = match index_kind {
            IndexKind::HashedNonUnique => quote!{
                pub(super) fn #remover_name(&mut self, key: &#ty) -> Vec<#element_name> {
                    if let Some(idxs) = self.#index_name.remove(key) {
                        let mut elems = Vec::with_capacity(idxs.len());
                        for idx in idxs {
                            let elem_orig = self._store.remove(idx);
                            #(#removes)*
                            elems.push(elem_orig)
                        }
                        elems
                    } else {
                        Vec::new()
                    }
                }  
            },
            _ => quote!{
                pub(super) fn #remover_name(&mut self, key: &#ty) -> Option<#element_name> {
                    let idx = self.#index_name.remove(key)?;
                    let elem_orig = self._store.remove(idx);
                    #(#removes)*
                    Some(elem_orig)
                }
            }
        };

        let modifier = match index_kind {
            IndexKind::HashedNonUnique => quote!{},
            _ => quote!{
                pub(super) fn #modifier_name(&mut self, key: &#ty, f: impl FnOnce(&mut #element_name)) -> Option<&#element_name> {
                    let elem = &mut self._store[*self.#index_name.get(key)?];
                    let elem_orig = elem.clone();
                    f(elem);
    
                    #(#modifies)*
    
                    Some(elem)
                }
            }
        };

        quote! {
            #getter

            #mut_getter

            #remover

            #modifier

            pub(super) fn #iter_getter_name(&mut self) -> #iter_name {
                #iter_name {
                    _store_ref: &self._store,
                    _iter: self.#index_name.iter(),
                    _inner_iter: None,
                }
            }
        }
    });

    // For each indexed field generate a TokenStream representing an accessor for the underlying storage via that field's lookup table
    let iterators = fields_to_index().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let iter_name = format_ident!(
            "{}{}Iter",
            map_name,
            field_name
                .to_string()
                .to_case(convert_case::Case::UpperCamel)
        );
        let ty = &f.ty;

        let index_kind = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });

        let iter_type = match index_kind {
            IndexKind::HashedUnique => {
                quote! {std::collections::hash_map::Iter<'a, #ty, usize>}
            }
            IndexKind::OrderedUnique => {
                quote! {std::collections::btree_map::Iter<'a, #ty, usize>}
            }
            IndexKind::HashedNonUnique => {
                quote! {std::collections::hash_map::Iter<'a, #ty, Vec<usize>>}
            }
        };

        let iter_action = match index_kind {
            IndexKind::HashedNonUnique => quote! {
                // If we have an inner_iter already, then get the next (optional) value from it.
                let inner_next = if let Some(inner_iter) = &mut self._inner_iter {
                    inner_iter.next()
                } else {
                    None
                };

                // If we have the next value, find it in the backing store.
                if let Some(next_index) = inner_next {
                    Some(&self._store_ref[*next_index])
                } else {
                    let hashmap_next = self._iter.next()?;
                    self._inner_iter = Some(hashmap_next.1.iter());
                    Some(&self._store_ref[*self._inner_iter.as_mut().unwrap().next().expect("Internal invariants broken, found empty slice in non_unique lookup table.")])
                }
            },
            _ => quote! { Some(&self._store_ref[*self._iter.next()?.1]) },
        };

        quote! {
            pub(super) struct #iter_name<'a> {
                _store_ref: &'a slab::Slab<#element_name>,
                _iter: #iter_type,
                _inner_iter: Option<core::slice::Iter<'a, usize>>,
            }

            impl<'a> Iterator for #iter_name<'a> {
                type Item = &'a #element_name;

                fn next(&mut self) -> Option<Self::Item> {
                    #iter_action
                }
            }
        }
    });

    // Build the final output using quasi-quoting
    let expanded = quote! {
        mod multi_index {
            use super::*;

            #[derive(Default, Clone)]
            pub(super) struct #map_name {
                _store: slab::Slab<#element_name>,
                #(#lookup_table_fields)*
            }

            impl #map_name {
                pub(super) fn len(&self) -> usize {
                    self._store.len()
                }

                pub(super) fn is_empty(&self) -> bool {
                    self._store.is_empty()
                }

                pub(super) fn insert(&mut self, elem: #element_name) {
                    let idx = self._store.insert(elem);
                    let elem = &self._store[idx];

                    #(#inserts)*
                }

                pub(super) fn iter(&self) -> slab::Iter<#element_name> {
                    self._store.iter()
                }

                // SAFETY:
                // It is safe to mutate the non-indexed fields, however mutating any of the indexed fields will break the internal invariants.
                // If the indexed fields need to be changed, the modify() method must be used.
                pub(super) unsafe fn iter_mut(&mut self) -> slab::IterMut<#element_name> {
                    self._store.iter_mut()
                }

                #(#accessors)*
            }

            #(#iterators)*
        }
    };

    // Hand the output tokens back to the compiler
    proc_macro::TokenStream::from(expanded)
}

// All these variants end in Unique, even "NonUnique", remove this warning.
#[allow(clippy::enum_variant_names)]
enum IndexKind {
    HashedUnique,
    OrderedUnique,
    HashedNonUnique,
}

fn get_index_kind(f: &syn::Field) -> Option<IndexKind> {
    let meta_list = match f.attrs.first()?.parse_meta() {
        Ok(syn::Meta::List(l)) => l,
        _ => return None,
    };

    let nested = meta_list.nested.first()?;

    let nested_path = match nested {
        syn::NestedMeta::Meta(syn::Meta::Path(p)) => p,
        _ => return None,
    };

    if nested_path.is_ident("hashed_unique") {
        Some(IndexKind::HashedUnique)
    } else if nested_path.is_ident("ordered_unique") {
        Some(IndexKind::OrderedUnique)
    } else if nested_path.is_ident("hashed_non_unique") {
        Some(IndexKind::HashedNonUnique)
    } else {
        None
    }
}