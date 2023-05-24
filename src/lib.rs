use convert_case::Casing;
use proc_macro_error::{abort_call_site, proc_macro_error};
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(MultiIndexMap, attributes(multi_index))]
#[proc_macro_error]
pub fn multi_index_map(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Extract the struct fields if we are parsing a struct, otherwise throw an error as we do not support Enums or Unions.
    let fields = match input.data {
        syn::Data::Struct(d) => d.fields,
        _ => abort_call_site!("MultiIndexMap only supports structs as elements"),
    };

    // Verify the struct fields are named fields, otherwise throw an error as we do not support Unnamed of Unit structs.
    let named_fields = match fields {
        syn::Fields::Named(f) => f,
        _ => abort_call_site!(
            "Struct fields must be named, unnamed tuple structs and unit structs are not supported"
        ),
    };

    // Filter out all the fields that do not have a multi_index attribute, so we can ignore the non-indexed fields.
    let fields_to_index = || {
        named_fields.named.iter().filter(|f| {
            f.attrs.first().is_some() && f.attrs.first().unwrap().path.is_ident("multi_index")
        })
    };

    // For each indexed field generate a TokenStream representing the lookup table for that field
    // Each lookup table maps it's index to a position in the backing storage,
    // or multiple positions in the backing storage in the non-unique indexes.
    let lookup_table_fields = fields_to_index().map(|f| {
        let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
        let ty = &f.ty;

        let (ordering, uniqueness) = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });

        match uniqueness {
            Uniqueness::Unique => match ordering {
                Ordering::Hashed => quote! {
                    #index_name: rustc_hash::FxHashMap<#ty, usize>,
                },
                Ordering::Ordered => quote! {
                    #index_name: std::collections::BTreeMap<#ty, usize>,
                }
            }
            Uniqueness::NonUnique => match ordering {
                Ordering::Hashed => quote! {
                    #index_name: rustc_hash::FxHashMap<#ty, std::collections::BTreeSet<usize>>,
                },
                Ordering::Ordered => quote! {
                    #index_name: std::collections::BTreeMap<#ty, std::collections::BTreeSet<usize>>,
                }
            }
        }
    });

    let lookup_table_fields_init: Vec<proc_macro2::TokenStream> = fields_to_index().map(|f|{
        let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
        let (ordering, _uniqueness) = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });
        match ordering {
            Ordering::Hashed => quote! {
                #index_name: rustc_hash::FxHashMap::default(),
            },
            Ordering::Ordered => quote! {
                #index_name: std::collections::BTreeMap::new(),
            }
        }
    }).collect();

    let lookup_table_fields_reserve: Vec<proc_macro2::TokenStream> = fields_to_index().map(|f|{
        let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
        let (ordering, _uniqueness) = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });

        match ordering {
            Ordering::Hashed => quote! {
                self.#index_name.reserve(additional);
            },
            Ordering::Ordered => quote! {}
        }

    }).collect();

    let lookup_table_fields_shrink: Vec<proc_macro2::TokenStream> = fields_to_index().map(|f|{
        let index_name = format_ident!("_{}_index", f.ident.as_ref().unwrap());
        let (ordering, _uniqueness) = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });

        match ordering {
            Ordering::Hashed => quote! {
                self.#index_name.shrink_to_fit();
            },
            Ordering::Ordered => quote! {}
        }

    }).collect();

    // For each indexed field generate a TokenStream representing inserting the position in the backing storage to that field's lookup table
    // Unique indexed fields just require a simple insert to the map, whereas non-unique fields require inserting to the Set of positions,
    // creating a new Set if necessary.
    let inserts: Vec<proc_macro2::TokenStream> = fields_to_index()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let field_name_string = field_name.to_string();
            let index_name = format_ident!("_{}_index", field_name);
            let (_ordering, uniqueness) = get_index_kind(f).unwrap_or_else(|| {
                abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
            });

            match uniqueness {
                Uniqueness::Unique => quote! { 
                    let orig_elem_idx = self.#index_name.insert(elem.#field_name.clone(), idx);
                    if orig_elem_idx.is_some() {
                        panic!("Unable to insert element, uniqueness constraint violated on field '{}'", #field_name_string);
                    }
                },
                Uniqueness::NonUnique => quote! {
                    self.#index_name.entry(elem.#field_name.clone()).or_insert(std::collections::BTreeSet::new()).insert(idx); 
                },
            }
        })
        .collect();

    /* 
        remove a given index from all fields, a reference to the element that is already deleted is given (elem_orig), the index of elem_orig in the backing storage before its removal is also given (idx)

        - when the field is unique, check that the index is indeed idx, then delete the corresponding key (elem_orig.#field_name) from the field
        - when the field is non-unique, get a reference to the Set that contains all back storage indices under the same key (elem_orig.#field_name), 
            - If there are more than one indices in the Set, remove idx from it
            - If there are exactly one index in the Set, then the index has to be idx, remove key and the entire Set
     */
    let removes: Vec<proc_macro2::TokenStream> = fields_to_index().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_name_string = field_name.to_string();
        let index_name = format_ident!("_{}_index", field_name);
        let error_msg = format!("Internal invariants broken, unable to find element in index '{field_name_string}' despite being present in another");
        let (_ordering, uniqueness) = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });

        match uniqueness {
            Uniqueness::Unique => quote! {
                // For unique indexes we know that removing an element will not affect any other elements
                // when the field is unique, delete the corresponding key (elem_orig.#field_name) from the field
                let removed_elem = self.#index_name.remove(&elem_orig.#field_name);
            },
            Uniqueness::NonUnique => quote! {
                // For non-unique indexes we must verify that we have not affected any other elements
                let key_to_remove = &elem_orig.#field_name;
                if let Some(mut elems) = self.#index_name.get_mut(key_to_remove) {
                    if elems.len() > 1 {
                        // If there are more than one indices in the Set, remove idx from it
                        if !elems.remove(&idx){
                            panic!(#error_msg);
                        }
                    } else {
                        // If there are exactly one index in the Set, then the index has to be idx, remove key and the entire Set
                        self.#index_name.remove(key_to_remove);
                    }
                }

            }
        }
    }).collect();


    // For each indexed field generate a TokenStream representing the combined remove and insert from that field's lookup table.
    /*
        The element before change is stored in reference `elem_orig`. the element after change is stored in reference `elem`. The index in the backing storage is `idx`

        for each field, only make changes if elem.#field_name and elem_orig.#field_name are not equal
            - when the field is unique, remove the old key and insert idx to the new key (if new key already exists, panic!)
            - when the field is non-unique, remove idx from the Set associaetd with the old key (if the Set is empty after removal, remove the old key), and insert idx to the new key

     */
    let modifies: Vec<proc_macro2::TokenStream> = fields_to_index().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_name_string = field_name.to_string();
        let index_name = format_ident!("_{}_index", field_name);
        let error_msg = format!("Internal invariants broken, unable to find element in index '{field_name_string}' despite being present in another");
        let (_ordering, uniqueness) = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });

        match uniqueness {
            Uniqueness::Unique => quote! {
                // only make changes if elem.#field_name and elem_orig.#field_name are not equal
                if elem.#field_name != elem_orig.#field_name {
                    let idx = self.#index_name.remove(&elem_orig.#field_name).expect(#error_msg);
                    let orig_elem_idx = self.#index_name.insert(elem.#field_name.clone(), idx);
                    if orig_elem_idx.is_some() {
                        panic!("Unable to insert element, uniqueness constraint violated on field '{}'", #field_name_string);
                    }
                }

            },
            Uniqueness::NonUnique => quote! {
                // only make changes if elem.#field_name and elem_orig.#field_name are not equal
                if elem.#field_name != elem_orig.#field_name {
                    // get the Set associated with the old key
                    let idxs = self.#index_name.get_mut(&elem_orig.#field_name).expect(#error_msg);
                    // remove idx from the Set
                    idxs.remove(&idx);
                    // insert idx to the new key
                    self.#index_name.entry(elem.#field_name.clone()).or_insert(std::collections::BTreeSet::new()).insert(idx); 
                }
            },
        }
    }).collect();

    let clears: Vec<proc_macro2::TokenStream> = fields_to_index()
        .map(|f| {
            let field_name = f.ident.as_ref().unwrap();
            let index_name = format_ident!("_{}_index", field_name);
            
            quote!{
                self.#index_name.clear();
            }
        })
        .collect();

    let element_name = input.ident;

    // Generate the name of the MultiIndexMap
    let map_name = format_ident!("MultiIndex{}Map", element_name);

    // For each indexed field generate a TokenStream representing all the accessors for the underlying storage via that field's lookup table.
    let accessors = fields_to_index().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_vis = &f.vis;
        let index_name = format_ident!("_{}_index", field_name);
        let getter_name = format_ident!("get_by_{}", field_name);
        let mut_getter_name = format_ident!("get_mut_by_{}", field_name);
        let remover_name = format_ident!("remove_by_{}", field_name);
        let modifier_name = format_ident!("modify_by_{}", field_name);
        let iter_name = format_ident!("{}{}Iter", map_name, field_name.to_string().to_case(convert_case::Case::UpperCamel));
        let iter_getter_name = format_ident!("iter_by_{}", field_name);
        let ty = &f.ty;
        let (_ordering, uniqueness) = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });

        // TokenStream representing the get_by_ accessor for this field.
        // For non-unique indexes we must go through all matching elements and find their positions,
        // in order to return a Vec of references to the backing storage.
        let getter = match uniqueness {
            Uniqueness::Unique => quote! {
                #field_vis fn #getter_name(&self, key: &#ty) -> Option<&#element_name> {
                    Some(&self._store[*self.#index_name.get(key)?])
                }
            },
            Uniqueness::NonUnique => quote! {
                #field_vis fn #getter_name(&self, key: &#ty) -> Vec<&#element_name> {
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
        };

        // TokenStream representing the get_mut_by_ accessor for this field.
        // Unavailable for NonUnique fields for now, because this would require returning multiple mutable references to the same backing storage.
        // This is not impossible to do safely, just requires some unsafe code and a thought out approach similar to split_at_mut.
        let mut_getter = match uniqueness {
            Uniqueness::Unique => quote! {
                // SAFETY:
                // It is safe to mutate the non-indexed fields, however mutating any of the indexed fields will break the internal invariants.
                // If the indexed fields need to be changed, the modify() method must be used.
                #field_vis unsafe fn #mut_getter_name(&mut self, key: &#ty) -> Option<&mut #element_name> {
                    Some(&mut self._store[*self.#index_name.get(key)?])
                }
            },
            Uniqueness::NonUnique => quote! {},
        };

        // TokenStream representing the remove_by_ accessor for this field.
        // For non-unique indexes we must go through all matching elements and find their positions,
        // in order to return a Vec elements from the backing storage.
        let remover = match uniqueness {
            Uniqueness::Unique => quote! {
                /*
                    When removing from a unique field:
                        - get the back storage index
                        - mark the index as unused in back storage
                        - remove the index from all fields
                        - return the element
                */ 
                #field_vis fn #remover_name(&mut self, key: &#ty) -> Option<#element_name> {
                    // get back storage index
                    let idx = self.#index_name.remove(key)?;
                    // mark the index as unused in back storage
                    let elem_orig = self._store.remove(idx);
                    // remove the index from all fields
                    #(#removes)*
                    // return the element
                    Some(elem_orig)
                }
            },
            Uniqueness::NonUnique => quote! {
                /*
                    When removing from a non-unique field:
                        - get the all back storage indices
                        - mark each index as unused in back storage
                        - remove each index from all fields
                        - return a Vec of elements
                */ 
                #field_vis fn #remover_name(&mut self, key: &#ty) -> Vec<#element_name> {
                    // get the all back storage indices
                    if let Some(idxs) = self.#index_name.remove(key) {
                        let mut elems = Vec::with_capacity(idxs.len());
                        for idx in idxs {
                            // mark all indices as unused in back storage
                            let elem_orig = self._store.remove(idx);
                            // remove the all indices from all fields
                            #(#removes)*
                            // push element into a Vec
                            elems.push(elem_orig)
                        }
                        // return the Vec
                        elems
                    } else {
                        Vec::new()
                    }
                }  
            },
        };

        // TokenStream representing the modify_by_ accessor for this field.
        // Unavailable for NonUnique fields for now, because the modification logic gets quite complicated.
        /*
            Given a key, update any field of the associated element
            - obtain a mutable reference of the element
            - apply changes to the reference
            - update all changed fields
            - return the modified item
         */
        let modifier = match uniqueness {
            Uniqueness::Unique => quote! {
                #field_vis fn #modifier_name(&mut self, key: &#ty, f: impl FnOnce(&mut #element_name)) -> Option<&#element_name> {
                    let idx = *self.#index_name.get(key)?;
                    // obtain a mutable reference of the element
                    let elem = &mut self._store[idx];
                    let elem_orig = elem.clone();
                    // apply changes to the reference
                    f(elem);
                    // update all changed fields
                    #(#modifies)*
                    // return the modified item
                    Some(elem)
                }
            },
            Uniqueness::NonUnique => quote! {},
        };

        // Put all these TokenStreams together, and put a TokenStream representing the iter_by_ accessor on the end.
        quote! {
            #getter

            #mut_getter

            #remover

            #modifier

            #field_vis fn #iter_getter_name(&self) -> #iter_name {
                #iter_name {
                    _store_ref: &self._store,
                    _iter: self.#index_name.iter(),
                    _inner_iter: None,
                }
            }
        }
    });

    // For each indexed field generate a TokenStream representing the Iterator over the backing storage via that field,
    // such that the elements are accessed in an order defined by the index rather than the backing storage.
    let iterators = fields_to_index().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_vis = &f.vis;
        let field_name_string = field_name.to_string();
        let error_msg = format!("Internal invariants broken, found empty slice in non_unique index '{field_name_string}'");
        let iter_name = format_ident!(
            "{}{}Iter",
            map_name,
            field_name
                .to_string()
                .to_case(convert_case::Case::UpperCamel)
        );
        let ty = &f.ty;

        let (ordering, uniqueness) = get_index_kind(f).unwrap_or_else(|| {
            abort_call_site!("Attributes must be in the style #[multi_index(hashed_unique)]")
        });

        // TokenStream representing the actual type of the iterator
        let iter_type = match uniqueness {
            Uniqueness::Unique => match ordering {
                Ordering::Hashed => quote! {std::collections::hash_map::Iter<'a, #ty, usize>},
                Ordering::Ordered => quote! {std::collections::btree_map::Iter<'a, #ty, usize>},
            }
            Uniqueness::NonUnique => match ordering {
                Ordering::Hashed => quote! {std::collections::hash_map::Iter<'a, #ty, std::collections::BTreeSet::<usize>>},
                Ordering::Ordered => quote! {std::collections::btree_map::Iter<'a, #ty, std::collections::BTreeSet::<usize>>},
            }
        };

        // TokenStream representing the logic for performing iteration.
        let iter_action = match uniqueness {
            Uniqueness::Unique => quote! { Some(&self._store_ref[*self._iter.next()?.1]) },
            Uniqueness::NonUnique => quote! {
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
                    self._inner_iter = Some(Box::new(hashmap_next.1.iter()));
                    Some(&self._store_ref[*self._inner_iter.as_mut().unwrap().next().expect(#error_msg)])
                }
            },
        };

        // TokenStream representing the iterator over each indexed field.
        // We have a different iterator type for each indexed field. Each one wraps the standard Iterator for that lookup table, but adds in a couple of things:
        // First we maintain a reference to the backing store, so we can return references to the elements we are interested in.
        // Second we maintain an optional inner_iter, only used for non-unique indexes. This is used to iterate through the Vec of matching elements for a given index value.
        quote! {
            #field_vis struct #iter_name<'a> {
                _store_ref: &'a slab::Slab<#element_name>,
                _iter: #iter_type,
                _inner_iter: Option<Box<dyn std::iter::Iterator<Item=&'a usize> +'a>>,
            }

            impl<'a> Iterator for #iter_name<'a> {
                type Item = &'a #element_name;

                fn next(&mut self) -> Option<Self::Item> {
                    #iter_action
                }
            }
        }
    });

    let element_vis = input.vis;

    // Build the final output using quasi-quoting
    let expanded = quote! {
        #[derive(Default, Clone)]
        #element_vis struct #map_name {
            _store: slab::Slab<#element_name>,
            #(#lookup_table_fields)*
        }

        impl #map_name {
            #element_vis fn with_capacity(n: usize) -> #map_name {
                #map_name {
                    _store: slab::Slab::with_capacity(n),
                    #(#lookup_table_fields_init)*
                }
            }

            #element_vis fn capacity(&self) -> usize {
                self._store.capacity()
            }

            #element_vis fn len(&self) -> usize {
                self._store.len()
            }

            #element_vis fn is_empty(&self) -> bool {
                self._store.is_empty()
            }

            #element_vis fn reserve(&mut self, additional: usize) {
                // reserving is slow. users are in control of when to reserve
                self._store.reserve(additional);
                #(#lookup_table_fields_reserve)* 
            }

            #element_vis fn shrink_to_fit(&mut self) {
                // shrinking is slow. users are in control of when to shrink
                self._store.shrink_to_fit();
                #(#lookup_table_fields_shrink)* 
            }

            #element_vis fn insert(&mut self, elem: #element_name) {
                let idx = self._store.insert(elem);
                let elem = &self._store[idx];

                #(#inserts)*
            }

            #element_vis fn clear(&mut self) {
                self._store.clear();
                #(#clears)*
            }

            // Allow iteration directly over the backing storage
            #element_vis fn iter(&self) -> slab::Iter<#element_name> {
                self._store.iter()
            }

            // SAFETY:
            // It is safe to mutate the non-indexed fields, however mutating any of the indexed fields will break the internal invariants.
            // If the indexed fields need to be changed, the modify() method must be used.
            #element_vis unsafe fn iter_mut(&mut self) -> slab::IterMut<#element_name> {
                self._store.iter_mut()
            }

            #(#accessors)*
        }

        #(#iterators)*
        
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

// Represents whether the index is Ordered or Hashed, ie. whether we use a BTreeMap or a FxHashMap as the lookup table.
enum Ordering {
    Hashed,
    Ordered,
}

// Represents whether the index is Unique or NonUnique, ie. whether we allow multiple elements with the same value in this index.
// All these variants end in Unique, even "NonUnique", remove this warning.
#[allow(clippy::enum_variant_names)]
enum Uniqueness {
    Unique,
    NonUnique,
}

// Get the Ordering and Uniqueness for a given field attribute.
fn get_index_kind(f: &syn::Field) -> Option<(Ordering, Uniqueness)> {
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
        Some((Ordering::Hashed, Uniqueness::Unique))
    } else if nested_path.is_ident("ordered_unique") {
        Some((Ordering::Ordered, Uniqueness::Unique))
    } else if nested_path.is_ident("hashed_non_unique") {
        Some((Ordering::Hashed, Uniqueness::NonUnique))
    } else if nested_path.is_ident("ordered_non_unique") {
        Some((Ordering::Ordered, Uniqueness::NonUnique))
    } else {
        None
    }
}