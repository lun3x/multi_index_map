use crate::model::{IndexedField, Input, Ordering, Uniqueness};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

pub(crate) fn generate(input: Input) -> TokenStream {
    let names = Names::new(&input);
    let fields = input
        .indexed
        .iter()
        .map(|field| FieldNames::new(&names, field))
        .collect::<Vec<_>>();

    let node_and_specs = generate_node_and_specs(&input, &names, &fields);
    let update = generate_update(&input, &names);
    let refs_and_iterators = generate_refs_and_iterators(&input, &names, &fields);
    let map = generate_map(&input, &names, &fields);
    let views = fields
        .iter()
        .map(|field| generate_view(&input, &names, field));

    quote! {
        #node_and_specs
        #update
        #refs_and_iterators
        #map
        #(#views)*
    }
}

struct Names {
    element: Ident,
    map: Ident,
    node: Ident,
    update: Ident,
    refs: Ident,
}

impl Names {
    fn new(input: &Input) -> Self {
        let element = input.element.clone();
        let map = format_ident!("MultiIndex{}Map", element);
        Self {
            node: format_ident!("__{}Node", map),
            update: format_ident!("{}Update", map),
            refs: format_ident!("__{}Refs", map),
            element,
            map,
        }
    }
}

struct FieldNames<'a> {
    field: &'a IndexedField,
    spec: Ident,
    link: Ident,
    index: Ident,
    view: Ident,
    view_mut: Ident,
    iter: Ident,
    equal_range: Ident,
    range: Ident,
    by: Ident,
    by_mut: Ident,
    get_by: Ident,
    get_mut_by: Ident,
    modify_by: Ident,
    update_by: Ident,
    remove_by: Ident,
    iter_by: Ident,
}

impl<'a> FieldNames<'a> {
    fn new(names: &Names, field: &'a IndexedField) -> Self {
        let field_ident = &field.ident;
        let camel = snake_to_camel(&field_ident.to_string());
        let map = &names.map;
        Self {
            field,
            spec: format_ident!("__{}By{}", map, camel),
            link: format_ident!("__mim_{}_link", field_ident),
            index: format_ident!("__mim_{}_index", field_ident),
            view: format_ident!("{}{}View", map, camel),
            view_mut: format_ident!("{}{}ViewMut", map, camel),
            iter: format_ident!("{}{}Iter", map, camel),
            equal_range: format_ident!("{}{}EqualRange", map, camel),
            range: format_ident!("{}{}Range", map, camel),
            by: format_ident!("by_{}", field_ident),
            by_mut: format_ident!("by_{}_mut", field_ident),
            get_by: format_ident!("get_by_{}", field_ident),
            get_mut_by: format_ident!("get_mut_by_{}", field_ident),
            modify_by: format_ident!("modify_by_{}", field_ident),
            update_by: format_ident!("update_by_{}", field_ident),
            remove_by: format_ident!("remove_by_{}", field_ident),
            iter_by: format_ident!("iter_by_{}", field_ident),
        }
    }

    fn unique(&self) -> bool {
        self.field.uniqueness == Uniqueness::Unique
    }

    fn ordered(&self) -> bool {
        self.field.ordering == Ordering::Ordered
    }

    fn index_type(&self, _node: &Ident) -> TokenStream {
        let spec = &self.spec;
        let unique = self.unique();
        match self.field.ordering {
            Ordering::Hashed => {
                quote!(::multi_index_map::__private::HashedIndex<#spec, #unique>)
            }
            Ordering::Ordered => {
                quote!(::multi_index_map::__private::OrderedIndex<#spec, #unique>)
            }
        }
    }

    fn ids_type(&self, node: &Ident) -> TokenStream {
        let spec = &self.spec;
        let unique = self.unique();
        match self.field.ordering {
            Ordering::Hashed => {
                quote!(::multi_index_map::__private::HashIds<'a, #node, #spec, #unique>)
            }
            Ordering::Ordered => {
                quote!(::multi_index_map::__private::OrderedIds<'a, #node, #spec, #unique>)
            }
        }
    }

    fn equal_ids_type(&self, node: &Ident) -> TokenStream {
        let spec = &self.spec;
        let unique = self.unique();
        match self.field.ordering {
            Ordering::Hashed => {
                quote!(::multi_index_map::__private::HashEqualIds<'a, #node, #spec, #unique>)
            }
            Ordering::Ordered => {
                quote!(::multi_index_map::__private::OrderedRangeIds<'a, #node, #spec, #unique>)
            }
        }
    }

    fn equal_range_name(&self) -> &Ident {
        if self.ordered() {
            &self.range
        } else {
            &self.equal_range
        }
    }

    fn query_bounds(&self, ty: &syn::Type) -> TokenStream {
        match self.field.ordering {
            Ordering::Hashed => quote! {
                #ty: ::std::borrow::Borrow<Q>,
                Q: ::std::hash::Hash + Eq + ?Sized,
            },
            Ordering::Ordered => quote! {
                #ty: ::std::borrow::Borrow<Q>,
                Q: Ord + ?Sized,
            },
        }
    }
}

fn snake_to_camel(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn generate_node_and_specs(
    _input: &Input,
    names: &Names,
    fields: &[FieldNames<'_>],
) -> TokenStream {
    let element = &names.element;
    let node = &names.node;
    let link_fields = fields.iter().map(|field| {
        let link = &field.link;
        match field.field.ordering {
            Ordering::Hashed => quote!(#link: ::multi_index_map::__private::HashLink),
            Ordering::Ordered => quote!(#link: ::multi_index_map::__private::OrderedLink),
        }
    });
    let link_defaults = fields.iter().map(|field| {
        let link = &field.link;
        quote!(#link: ::std::default::Default::default())
    });
    let specs = fields.iter().map(|field| {
        let spec = &field.spec;
        let field_ident = &field.field.ident;
        let ty = &field.field.ty;
        let link = &field.link;
        let name = field_ident.to_string();
        match field.field.ordering {
            Ordering::Hashed => quote! {
                struct #spec;

                impl ::multi_index_map::__private::HashSpec<#node> for #spec {
                    type Key = #ty;
                    const NAME: &'static str = #name;

                    fn key(value: &#element) -> &Self::Key {
                        &value.#field_ident
                    }

                    fn link(node: &#node) -> &::multi_index_map::__private::HashLink {
                        &node.#link
                    }

                    fn link_mut(node: &mut #node) -> &mut ::multi_index_map::__private::HashLink {
                        &mut node.#link
                    }
                }
            },
            Ordering::Ordered => quote! {
                struct #spec;

                impl ::multi_index_map::__private::OrderedSpec<#node> for #spec {
                    type Key = #ty;
                    const NAME: &'static str = #name;

                    fn key(value: &#element) -> &Self::Key {
                        &value.#field_ident
                    }

                    fn link(node: &#node) -> &::multi_index_map::__private::OrderedLink {
                        &node.#link
                    }

                    fn link_mut(node: &mut #node) -> &mut ::multi_index_map::__private::OrderedLink {
                        &mut node.#link
                    }
                }
            },
        }
    });

    quote! {
        struct #node {
            value: #element,
            #(#link_fields,)*
        }

        impl #node {
            fn new(value: #element) -> Self {
                Self {
                    value,
                    #(#link_defaults,)*
                }
            }
        }

        impl ::multi_index_map::__private::NodeValue for #node {
            type Value = #element;

            fn value(&self) -> &Self::Value {
                &self.value
            }
        }

        #(#specs)*
    }
}

fn generate_update(input: &Input, names: &Names) -> TokenStream {
    let vis = &input.vis;
    let update = &names.update;
    let element = &names.element;
    if input.unindexed.is_empty() {
        quote! {
            #[allow(dead_code)]
            #vis struct #update<'a> {
                #[doc(hidden)]
                pub _marker: ::std::marker::PhantomData<&'a mut #element>,
            }
        }
    } else {
        let fields = input.unindexed.iter().map(|field| {
            let vis = &field.vis;
            let ident = &field.ident;
            let ty = &field.ty;
            quote!(#vis #ident: &'a mut #ty)
        });
        quote! {
            #[allow(dead_code)]
            #vis struct #update<'a> {
                #(#fields,)*
            }
        }
    }
}

fn generate_refs_and_iterators(
    _input: &Input,
    names: &Names,
    fields: &[FieldNames<'_>],
) -> TokenStream {
    let refs = &names.refs;
    let node = &names.node;
    let element = &names.element;
    let wrappers = fields
        .iter()
        .map(|field| generate_iterator_wrappers(names, field));

    quote! {
        struct #refs<'a, I> {
            nodes: &'a ::multi_index_map::__private::Slab<#node>,
            ids: I,
        }

        impl<'a, I> #refs<'a, I> {
            fn new(nodes: &'a ::multi_index_map::__private::Slab<#node>, ids: I) -> Self {
                Self { nodes, ids }
            }
        }

        impl<'a, I> Iterator for #refs<'a, I>
        where
            I: Iterator<Item = ::multi_index_map::__private::NodeId>,
        {
            type Item = &'a #element;

            fn next(&mut self) -> Option<Self::Item> {
                self.ids.next().map(|id| &self.nodes[id.0].value)
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.ids.size_hint()
            }
        }

        impl<I> DoubleEndedIterator for #refs<'_, I>
        where
            I: DoubleEndedIterator<Item = ::multi_index_map::__private::NodeId>,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.ids.next_back().map(|id| &self.nodes[id.0].value)
            }
        }

        impl<I> ExactSizeIterator for #refs<'_, I>
        where
            I: ExactSizeIterator<Item = ::multi_index_map::__private::NodeId>,
        {}

        impl<I> ::std::iter::FusedIterator for #refs<'_, I>
        where
            I: ::std::iter::FusedIterator<Item = ::multi_index_map::__private::NodeId>,
        {}

        #(#wrappers)*
    }
}

fn generate_iterator_wrappers(names: &Names, field: &FieldNames<'_>) -> TokenStream {
    let refs = &names.refs;
    let element = &names.element;
    let node = &names.node;
    let vis = &field.field.vis;
    let iter = &field.iter;
    let ids_ty = field.ids_type(node);

    let iter_double = field.ordered().then(|| {
        quote! {
            impl DoubleEndedIterator for #iter<'_> {
                fn next_back(&mut self) -> Option<Self::Item> {
                    self.inner.next_back()
                }
            }
        }
    });

    let secondary = if field.ordered() {
        let range = &field.range;
        let range_ty = field.equal_ids_type(node);
        quote! {
            #vis struct #range<'a> {
                inner: #refs<'a, #range_ty>,
            }

            impl<'a> #range<'a> {
                fn new(inner: #refs<'a, #range_ty>) -> Self {
                    Self { inner }
                }
            }

            impl<'a> Iterator for #range<'a> {
                type Item = &'a #element;
                fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
                fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
            }

            impl DoubleEndedIterator for #range<'_> {
                fn next_back(&mut self) -> Option<Self::Item> { self.inner.next_back() }
            }

            impl ::std::iter::FusedIterator for #range<'_> {}
        }
    } else if !field.unique() {
        let equal_range = field.equal_range_name();
        let equal_ty = field.equal_ids_type(node);
        quote! {
            #vis struct #equal_range<'a> {
                inner: #refs<'a, #equal_ty>,
            }

            impl<'a> #equal_range<'a> {
                fn new(inner: #refs<'a, #equal_ty>) -> Self {
                    Self { inner }
                }
            }

            impl<'a> Iterator for #equal_range<'a> {
                type Item = &'a #element;
                fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
                fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
            }

            impl ExactSizeIterator for #equal_range<'_> {}
            impl ::std::iter::FusedIterator for #equal_range<'_> {}
        }
    } else {
        TokenStream::new()
    };

    quote! {
        #vis struct #iter<'a> {
            inner: #refs<'a, #ids_ty>,
        }

        impl<'a> #iter<'a> {
            fn new(inner: #refs<'a, #ids_ty>) -> Self {
                Self { inner }
            }
        }

        impl<'a> Iterator for #iter<'a> {
            type Item = &'a #element;
            fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
            fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
        }

        #iter_double
        impl ExactSizeIterator for #iter<'_> {}
        impl ::std::iter::FusedIterator for #iter<'_> {}

        #secondary
    }
}

fn generate_map(input: &Input, names: &Names, fields: &[FieldNames<'_>]) -> TokenStream {
    let vis = &input.vis;
    let element = &names.element;
    let map = &names.map;
    let node = &names.node;
    let update = &names.update;

    let index_fields = fields.iter().map(|field| {
        let index = &field.index;
        let index_ty = field.index_type(node);
        quote!(#index: #index_ty)
    });
    let index_defaults = fields.iter().map(|field| {
        let index = &field.index;
        quote!(#index: ::std::default::Default::default())
    });
    let view_methods = fields.iter().map(|field| {
        let field_vis = &field.field.vis;
        let by = &field.by;
        let by_mut = &field.by_mut;
        let view = &field.view;
        let view_mut = &field.view_mut;
        quote! {
            #field_vis fn #by(&self) -> #view<'_> {
                #view { map: self }
            }

            #field_vis fn #by_mut(&mut self) -> #view_mut<'_> {
                #view_mut { map: self }
            }
        }
    });

    let unique_checks = fields.iter().filter(|field| field.unique()).map(|field| {
        let index = &field.index;
        let field_ident = &field.field.ident;
        let name = field_ident.to_string();
        quote! {
            if self.#index.find(&value.#field_ident, &self.nodes).is_some() {
                return Err(::multi_index_map::Conflict { index: #name, value });
            }
        }
    });
    let hash_reserves = fields
        .iter()
        .filter(|field| field.field.ordering == Ordering::Hashed)
        .map(|field| {
            let index = &field.index;
            quote!(self.#index.reserve_for_insert(&mut self.nodes);)
        });
    let link_all = fields.iter().map(|field| {
        let index = &field.index;
        let name = field.field.ident.to_string();
        quote! {
            self.#index
                .insert(id, &mut self.nodes)
                .unwrap_or_else(|_| panic!("prepared insertion unexpectedly conflicted on index '{}'", #name));
        }
    });
    let unlink_all = fields.iter().map(|field| {
        let index = &field.index;
        quote!(self.#index.remove(id, &mut self.nodes);)
    });
    let replace_checks = fields.iter().filter(|field| field.unique()).map(|field| {
        let index = &field.index;
        let field_ident = &field.field.ident;
        let name = field_ident.to_string();
        quote! {
            if self.#index
                .find(&replacement.#field_ident, &self.nodes)
                .is_some_and(|other| other != id)
            {
                return Err(::multi_index_map::Conflict { index: #name, value: replacement });
            }
        }
    });
    let reconciles = fields.iter().map(|field| {
        let index = &field.index;
        let name = field.field.ident.to_string();
        quote! {
            if conflict.is_none() && self.#index.reconcile(id, &mut self.nodes).is_err() {
                conflict = Some(#name);
            }
        }
    });
    let validates = fields.iter().map(|field| {
        let index = &field.index;
        quote!(self.#index.validate(&self.nodes)?;)
    });
    let lengths = fields.iter().map(|field| {
        let index = &field.index;
        quote!(self.#index.len())
    });

    let update_proxy = update_proxy_expr(input, names);
    let update_fields_helper = generate_update_fields_helper(input, names);
    let compatibility = fields
        .iter()
        .map(|field| generate_compatibility_method(input, names, field));

    quote! {
        #vis struct #map {
            nodes: ::multi_index_map::__private::Slab<#node>,
            #(#index_fields,)*
        }

        impl Default for #map {
            fn default() -> Self {
                Self {
                    nodes: ::std::default::Default::default(),
                    #(#index_defaults,)*
                }
            }
        }

        impl #map {
            #vis fn new() -> Self {
                Self::default()
            }

            #vis fn len(&self) -> usize {
                self.nodes.len()
            }

            #vis fn is_empty(&self) -> bool {
                self.nodes.is_empty()
            }

            #vis fn try_insert(
                &mut self,
                value: #element,
            ) -> Result<&#element, ::multi_index_map::Conflict<#element>> {
                #(#unique_checks)*
                #(#hash_reserves)*
                let id = ::multi_index_map::__private::NodeId(self.nodes.insert(#node::new(value)));
                self.link_all(id);
                self.validate_debug();
                Ok(&self.nodes[id.0].value)
            }

            #vis fn insert(&mut self, value: #element) -> &#element {
                match self.try_insert(value) {
                    Ok(value) => value,
                    Err(conflict) => panic!(
                        "unable to insert element: uniqueness constraint violated on index '{}'",
                        conflict.index
                    ),
                }
            }

            #vis fn clear(&mut self) {
                let ids = self
                    .nodes
                    .iter()
                    .map(|(id, _)| ::multi_index_map::__private::NodeId(id))
                    .collect::<Vec<_>>();
                for id in ids {
                    self.remove_id(id);
                }
                self.validate_debug();
            }

            #(#view_methods)*

            #update_fields_helper

            fn order_refs_for_ids(
                &self,
                ids: &[::multi_index_map::__private::NodeId],
            ) -> Vec<&#element> {
                ids.iter()
                    .map(|id| {
                        &self
                            .nodes
                            .get(id.0)
                            .expect("compatibility accessor targeted a missing arena node")
                            .value
                    })
                    .collect()
            }

            fn panic_on_modify_conflicts(result: ::multi_index_map::ModifyAllResult<#element>) {
                if let Some(conflict) = result.removed.first() {
                    panic!(
                        "compatibility modifier removed {} element(s) after uniqueness conflict on index '{}'",
                        result.removed.len(),
                        conflict.index
                    );
                }
            }

            #(#compatibility)*

            fn link_all(&mut self, id: ::multi_index_map::__private::NodeId) {
                #(#link_all)*
            }

            fn unlink_all(&mut self, id: ::multi_index_map::__private::NodeId) {
                #(#unlink_all)*
            }

            fn remove_id(&mut self, id: ::multi_index_map::__private::NodeId) -> #element {
                self.unlink_all(id);
                self.nodes.remove(id.0).value
            }

            fn replace_id(
                &mut self,
                id: ::multi_index_map::__private::NodeId,
                replacement: #element,
            ) -> Result<#element, ::multi_index_map::Conflict<#element>> {
                #(#replace_checks)*
                self.unlink_all(id);
                let old = ::std::mem::replace(&mut self.nodes[id.0].value, replacement);
                self.link_all(id);
                self.validate_debug();
                Ok(old)
            }

            fn modify_id(
                &mut self,
                id: ::multi_index_map::__private::NodeId,
                f: impl FnOnce(&mut #element),
            ) -> Result<&#element, ::multi_index_map::Conflict<#element>> {
                let result = ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| {
                    f(&mut self.nodes[id.0].value)
                }));
                if let Err(payload) = result {
                    self.remove_id(id);
                    self.validate_debug();
                    ::std::panic::resume_unwind(payload);
                }

                let mut conflict = None;
                #(#reconciles)*
                if let Some(index) = conflict {
                    let value = self.remove_id(id);
                    self.validate_debug();
                    return Err(::multi_index_map::Conflict { index, value });
                }
                self.validate_debug();
                Ok(&self.nodes[id.0].value)
            }

            fn modify_ids(
                &mut self,
                ids: Vec<::multi_index_map::__private::NodeId>,
                mut f: impl FnMut(&mut #element),
            ) -> ::multi_index_map::ModifyAllResult<#element> {
                let mut result = ::multi_index_map::ModifyAllResult::default();
                for id in ids {
                    if !self.nodes.contains(id.0) {
                        continue;
                    }
                    match self.modify_id(id, &mut f) {
                        Ok(_) => result.modified += 1,
                        Err(conflict) => result.removed.push(conflict),
                    }
                }
                result
            }

            fn update_id(
                &mut self,
                id: ::multi_index_map::__private::NodeId,
                f: impl FnOnce(#update<'_>),
            ) -> &#element {
                let value = &mut self.nodes[id.0].value;
                f(#update_proxy);
                &self.nodes[id.0].value
            }

            fn update_ids(
                &mut self,
                ids: Vec<::multi_index_map::__private::NodeId>,
                mut f: impl FnMut(#update<'_>),
            ) -> usize {
                for id in &ids {
                    let value = &mut self.nodes[id.0].value;
                    f(#update_proxy);
                }
                ids.len()
            }

            #vis fn validate(&self) -> Result<(), String> {
                #(#validates)*
                let len = self.nodes.len();
                if [#(#lengths,)*].into_iter().any(|index_len| index_len != len) {
                    return Err("an index count differs from the arena length".to_string());
                }
                Ok(())
            }

            fn validate_debug(&self) {
                debug_assert!(self.validate().is_ok(), "{:?}", self.validate());
            }
        }
    }
}

fn update_proxy_expr(input: &Input, names: &Names) -> TokenStream {
    let update = &names.update;
    if input.unindexed.is_empty() {
        quote!(#update { _marker: ::std::marker::PhantomData })
    } else {
        let fields = input.unindexed.iter().map(|field| {
            let ident = &field.ident;
            quote!(#ident: &mut value.#ident)
        });
        quote!(#update { #(#fields,)* })
    }
}

fn generate_update_fields_helper(input: &Input, _names: &Names) -> TokenStream {
    let tuple_type = update_tuple_type(input);
    if input.unindexed.is_empty() {
        quote! {
            fn update_fields_for_ids(
                &mut self,
                mut ids: Vec<::multi_index_map::__private::NodeId>,
            ) -> Vec<#tuple_type> {
                ids.sort_unstable_by_key(|id| id.0);
                assert!(
                    !ids.windows(2).any(|pair| pair[0] == pair[1]),
                    "compatibility accessor received a duplicate arena node"
                );
                for id in &ids {
                    assert!(
                        self.nodes.contains(id.0),
                        "compatibility accessor targeted a missing arena node"
                    );
                }
                vec![(); ids.len()]
            }
        }
    } else {
        let tuple_values = input.unindexed.iter().map(|field| {
            let ident = &field.ident;
            quote!(&mut node.value.#ident)
        });
        quote! {
            fn update_fields_for_ids(
                &mut self,
                mut ids: Vec<::multi_index_map::__private::NodeId>,
            ) -> Vec<#tuple_type> {
                ids.sort_unstable_by_key(|id| id.0);
                assert!(
                    !ids.windows(2).any(|pair| pair[0] == pair[1]),
                    "compatibility accessor received a duplicate arena node"
                );

                let mut fields = Vec::with_capacity(ids.len());
                let mut targets = ids.into_iter();
                let mut target = targets.next();
                for (slot, node) in self.nodes.iter_mut() {
                    if target.map(|id| id.0) == Some(slot) {
                        fields.push((#(#tuple_values,)*));
                        target = targets.next();
                    }
                }
                assert!(
                    target.is_none(),
                    "compatibility accessor targeted a missing arena node"
                );
                fields
            }
        }
    }
}

fn update_tuple_type(input: &Input) -> TokenStream {
    let types = input.unindexed.iter().map(|field| {
        let ty = &field.ty;
        quote!(&mut #ty)
    });
    quote!((#(#types,)*))
}

fn update_proxy_call_args(input: &Input) -> TokenStream {
    let fields = input.unindexed.iter().map(|field| {
        let ident = &field.ident;
        quote!(fields.#ident)
    });
    quote!(#(#fields,)*)
}

fn generate_compatibility_method(
    input: &Input,
    names: &Names,
    field: &FieldNames<'_>,
) -> TokenStream {
    let element = &names.element;
    let refs = &names.refs;
    let field_vis = &field.field.vis;
    let ty = &field.field.ty;
    let index = &field.index;
    let get_by = &field.get_by;
    let get_mut_by = &field.get_mut_by;
    let modify_by = &field.modify_by;
    let update_by = &field.update_by;
    let remove_by = &field.remove_by;
    let iter_by = &field.iter_by;
    let iter = &field.iter;
    let tuple_type = update_tuple_type(input);
    let query_bounds = field.query_bounds(ty);
    let update_args = update_proxy_call_args(input);
    let update_arg_types = input.unindexed.iter().map(|field| {
        let ty = &field.ty;
        quote!(&mut #ty)
    });
    let update_arg_types = update_arg_types.collect::<Vec<_>>();

    let get = if field.unique() {
        quote! {
            #[deprecated(note = "use the corresponding view's get method")]
            #field_vis fn #get_by<Q>(&self, key: &Q) -> Option<&#element>
            where
                #query_bounds
            {
                self.#index.find(key, &self.nodes).map(|id| &self.nodes[id.0].value)
            }
        }
    } else {
        quote! {
            #[deprecated(note = "use the corresponding view's equal_range method")]
            #field_vis fn #get_by<Q>(&self, key: &Q) -> Vec<&#element>
            where
                #query_bounds
            {
                self.#index
                    .equal_ids(key, &self.nodes)
                    .into_iter()
                    .map(|id| &self.nodes[id.0].value)
                    .collect()
            }
        }
    };

    let get_mut_body = if field.unique() {
        quote! {
            let id = self.#index.find(key, &self.nodes)?;
            self.update_fields_for_ids(vec![id]).into_iter().next()
        }
    } else {
        quote! {
            let ids = self.#index.equal_ids(key, &self.nodes);
            self.update_fields_for_ids(ids)
        }
    };
    let get_mut_return = if field.unique() {
        quote!(Option<#tuple_type>)
    } else {
        quote!(Vec<#tuple_type>)
    };

    let modify = if field.unique() {
        quote! {
            #[deprecated(note = "use the corresponding mutable view's modify method")]
            #field_vis fn #modify_by(
                &mut self,
                key: &#ty,
                f: impl FnOnce(&mut #element),
            ) -> Option<&#element> {
                let id = self.#index.find(key, &self.nodes)?;
                match self.modify_id(id, f) {
                    Ok(value) => Some(value),
                    Err(conflict) => panic!(
                        "compatibility modifier removed an element after uniqueness conflict on index '{}'",
                        conflict.index
                    ),
                }
            }
        }
    } else {
        quote! {
            #[deprecated(note = "use the corresponding mutable view's modify_all method")]
            #field_vis fn #modify_by(
                &mut self,
                key: &#ty,
                f: impl FnMut(&mut #element),
            ) -> Vec<&#element> {
                let ids = self.#index.equal_ids(key, &self.nodes);
                let result = self.modify_ids(ids.clone(), f);
                Self::panic_on_modify_conflicts(result);
                self.order_refs_for_ids(&ids)
            }
        }
    };

    let update = if field.unique() {
        quote! {
            #[deprecated(note = "use the corresponding mutable view's update method")]
            #field_vis fn #update_by<Q>(
                &mut self,
                key: &Q,
                f: impl FnOnce(#(#update_arg_types),*),
            ) -> Option<&#element>
            where
                #query_bounds
            {
                let id = self.#index.find(key, &self.nodes)?;
                Some(self.update_id(id, |fields| f(#update_args)))
            }
        }
    } else {
        quote! {
            #[deprecated(note = "use the corresponding mutable view's update_all method")]
            #field_vis fn #update_by<Q>(
                &mut self,
                key: &Q,
                mut f: impl FnMut(#(#update_arg_types),*),
            ) -> Vec<&#element>
            where
                #query_bounds
            {
                let ids = self.#index.equal_ids(key, &self.nodes);
                for id in &ids {
                    self.update_id(*id, |fields| f(#update_args));
                }
                self.order_refs_for_ids(&ids)
            }
        }
    };

    let remove = if field.unique() {
        quote! {
            #[deprecated(note = "use the corresponding mutable view's remove method")]
            #field_vis fn #remove_by(&mut self, key: &#ty) -> Option<#element> {
                let id = self.#index.find(key, &self.nodes)?;
                let value = self.remove_id(id);
                self.validate_debug();
                Some(value)
            }
        }
    } else {
        quote! {
            #[deprecated(note = "use the corresponding mutable view's remove_all method")]
            #field_vis fn #remove_by(&mut self, key: &#ty) -> Vec<#element> {
                let ids = self.#index.equal_ids(key, &self.nodes);
                let values = ids.into_iter().map(|id| self.remove_id(id)).collect();
                self.validate_debug();
                values
            }
        }
    };

    quote! {
        #get

        #[deprecated(note = "use the corresponding mutable view's update method")]
        #field_vis fn #get_mut_by(&mut self, key: &#ty) -> #get_mut_return {
            #get_mut_body
        }

        #modify
        #update
        #remove

        #[deprecated(note = "use the corresponding view's iter method")]
        #field_vis fn #iter_by(&self) -> #iter<'_> {
            #iter::new(#refs::new(&self.nodes, self.#index.iter_ids(&self.nodes)))
        }
    }
}

fn generate_view(_input: &Input, names: &Names, field: &FieldNames<'_>) -> TokenStream {
    let element = &names.element;
    let map = &names.map;
    let update = &names.update;
    let refs = &names.refs;
    let vis = &field.field.vis;
    let ty = &field.field.ty;
    let index = &field.index;
    let view = &field.view;
    let view_mut = &field.view_mut;
    let iter = &field.iter;
    let query_bounds = field.query_bounds(ty);

    let immutable_category = if field.unique() {
        quote! {
            #vis fn get<Q>(&self, key: &Q) -> Option<&'a #element>
            where
                #query_bounds
            {
                self.map
                    .#index
                    .find(key, &self.map.nodes)
                    .map(|id| &self.map.nodes[id.0].value)
            }

            #vis fn contains_key<Q>(&self, key: &Q) -> bool
            where
                #query_bounds
            {
                self.map.#index.find(key, &self.map.nodes).is_some()
            }
        }
    } else {
        let equal_range = field.equal_range_name();
        quote! {
            #vis fn equal_range<Q>(&self, key: &Q) -> #equal_range<'a>
            where
                #query_bounds
            {
                #equal_range::new(#refs::new(
                    &self.map.nodes,
                    self.map.#index.equal_iter_ids(key, &self.map.nodes),
                ))
            }
        }
    };

    let mutable_read_category = if field.unique() {
        quote! {
            #vis fn get<Q>(&self, key: &Q) -> Option<&#element>
            where
                #query_bounds
            {
                self.map
                    .#index
                    .find(key, &self.map.nodes)
                    .map(|id| &self.map.nodes[id.0].value)
            }

            #vis fn contains_key<Q>(&self, key: &Q) -> bool
            where
                #query_bounds
            {
                self.map.#index.find(key, &self.map.nodes).is_some()
            }
        }
    } else {
        let equal_range = field.equal_range_name();
        quote! {
            #vis fn equal_range<Q>(&self, key: &Q) -> #equal_range<'_>
            where
                #query_bounds
            {
                #equal_range::new(#refs::new(
                    &self.map.nodes,
                    self.map.#index.equal_iter_ids(key, &self.map.nodes),
                ))
            }
        }
    };

    let immutable_range = field.ordered().then(|| {
        let range = &field.range;
        quote! {
            #vis fn range<R>(&self, range: R) -> #range<'a>
            where
                R: ::std::ops::RangeBounds<#ty>,
            {
                #range::new(#refs::new(
                    &self.map.nodes,
                    self.map.#index.range_iter_ids(range, &self.map.nodes),
                ))
            }
        }
    });

    let mutable_range = field.ordered().then(|| {
        let range = &field.range;
        quote! {
            #vis fn range<R>(&self, range: R) -> #range<'_>
            where
                R: ::std::ops::RangeBounds<#ty>,
            {
                #range::new(#refs::new(
                    &self.map.nodes,
                    self.map.#index.range_iter_ids(range, &self.map.nodes),
                ))
            }
        }
    });

    let mutations = if field.unique() {
        quote! {
            #vis fn remove<Q>(&mut self, key: &Q) -> Option<#element>
            where
                #query_bounds
            {
                let id = self.map.#index.find(key, &self.map.nodes)?;
                let value = self.map.remove_id(id);
                self.map.validate_debug();
                Some(value)
            }

            #vis fn replace<Q>(
                &mut self,
                key: &Q,
                replacement: #element,
            ) -> Result<Option<#element>, ::multi_index_map::Conflict<#element>>
            where
                #query_bounds
            {
                let Some(id) = self.map.#index.find(key, &self.map.nodes) else {
                    return Ok(None);
                };
                self.map.replace_id(id, replacement).map(Some)
            }

            #vis fn modify<Q>(
                &mut self,
                key: &Q,
                f: impl FnOnce(&mut #element),
            ) -> Result<Option<&#element>, ::multi_index_map::Conflict<#element>>
            where
                #query_bounds
            {
                let Some(id) = self.map.#index.find(key, &self.map.nodes) else {
                    return Ok(None);
                };
                self.map.modify_id(id, f).map(Some)
            }

            #vis fn update<Q>(
                &mut self,
                key: &Q,
                f: impl FnOnce(#update<'_>),
            ) -> Option<&#element>
            where
                #query_bounds
            {
                let id = self.map.#index.find(key, &self.map.nodes)?;
                Some(self.map.update_id(id, f))
            }
        }
    } else {
        quote! {
            #vis fn remove_all<Q>(&mut self, key: &Q) -> Vec<#element>
            where
                #query_bounds
            {
                let ids = self.map.#index.equal_ids(key, &self.map.nodes);
                let values = ids
                    .into_iter()
                    .map(|id| self.map.remove_id(id))
                    .collect();
                self.map.validate_debug();
                values
            }

            #vis fn modify_all<Q>(
                &mut self,
                key: &Q,
                f: impl FnMut(&mut #element),
            ) -> ::multi_index_map::ModifyAllResult<#element>
            where
                #query_bounds
            {
                let ids = self.map.#index.equal_ids(key, &self.map.nodes);
                self.map.modify_ids(ids, f)
            }

            #vis fn update_all<Q>(
                &mut self,
                key: &Q,
                f: impl FnMut(#update<'_>),
            ) -> usize
            where
                #query_bounds
            {
                let ids = self.map.#index.equal_ids(key, &self.map.nodes);
                self.map.update_ids(ids, f)
            }
        }
    };

    let unique_traits = field.unique().then(|| {
        quote! {
            impl ::multi_index_map::UniqueView for #view<'_> {
                fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
                    self.map
                        .#index
                        .find(key, &self.map.nodes)
                        .map(|id| &self.map.nodes[id.0].value)
                }
            }

            impl ::multi_index_map::UniqueView for #view_mut<'_> {
                fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
                    self.map
                        .#index
                        .find(key, &self.map.nodes)
                        .map(|id| &self.map.nodes[id.0].value)
                }
            }

            impl ::multi_index_map::UniqueViewMut for #view_mut<'_> {
                type Conflict = ::multi_index_map::Conflict<#element>;
                type Update<'a> = #update<'a>;

                fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
                    #view_mut::remove(self, key)
                }

                fn replace(
                    &mut self,
                    key: &Self::Key,
                    replacement: Self::Value,
                ) -> Result<Option<Self::Value>, Self::Conflict> {
                    #view_mut::replace(self, key, replacement)
                }

                fn modify<F>(
                    &mut self,
                    key: &Self::Key,
                    f: F,
                ) -> Result<Option<&Self::Value>, Self::Conflict>
                where
                    F: FnOnce(&mut Self::Value),
                {
                    #view_mut::modify(self, key, f)
                }

                fn update<F>(&mut self, key: &Self::Key, f: F) -> Option<&Self::Value>
                where
                    F: for<'a> FnOnce(Self::Update<'a>),
                {
                    #view_mut::update(self, key, f)
                }
            }
        }
    });

    let non_unique_traits = (!field.unique()).then(|| {
        let equal_range = field.equal_range_name();
        quote! {
            impl ::multi_index_map::NonUniqueView for #view<'_> {
                type EqualRange<'a> = #equal_range<'a> where Self: 'a;

                fn equal_range(&self, key: &Self::Key) -> Self::EqualRange<'_> {
                    #equal_range::new(#refs::new(
                        &self.map.nodes,
                        self.map.#index.equal_iter_ids(key, &self.map.nodes),
                    ))
                }
            }

            impl ::multi_index_map::NonUniqueView for #view_mut<'_> {
                type EqualRange<'a> = #equal_range<'a> where Self: 'a;

                fn equal_range(&self, key: &Self::Key) -> Self::EqualRange<'_> {
                    #equal_range::new(#refs::new(
                        &self.map.nodes,
                        self.map.#index.equal_iter_ids(key, &self.map.nodes),
                    ))
                }
            }

            impl ::multi_index_map::NonUniqueViewMut for #view_mut<'_> {
                type ModifyAllResult = ::multi_index_map::ModifyAllResult<#element>;
                type Update<'a> = #update<'a>;

                fn remove_all(&mut self, key: &Self::Key) -> Vec<Self::Value> {
                    #view_mut::remove_all(self, key)
                }

                fn modify_all<F>(&mut self, key: &Self::Key, f: F) -> Self::ModifyAllResult
                where
                    F: FnMut(&mut Self::Value),
                {
                    #view_mut::modify_all(self, key, f)
                }

                fn update_all<F>(&mut self, key: &Self::Key, f: F) -> usize
                where
                    F: for<'a> FnMut(Self::Update<'a>),
                {
                    #view_mut::update_all(self, key, f)
                }
            }
        }
    });

    let ordered_traits = field.ordered().then(|| {
        let range = &field.range;
        quote! {
            impl ::multi_index_map::OrderedView for #view<'_> {
                type Range<'a> = #range<'a> where Self: 'a;

                fn range<R>(&self, range: R) -> Self::Range<'_>
                where
                    R: ::std::ops::RangeBounds<Self::Key>,
                {
                    #range::new(#refs::new(
                        &self.map.nodes,
                        self.map.#index.range_iter_ids(range, &self.map.nodes),
                    ))
                }
            }

            impl ::multi_index_map::OrderedView for #view_mut<'_> {
                type Range<'a> = #range<'a> where Self: 'a;

                fn range<R>(&self, range: R) -> Self::Range<'_>
                where
                    R: ::std::ops::RangeBounds<Self::Key>,
                {
                    #range::new(#refs::new(
                        &self.map.nodes,
                        self.map.#index.range_iter_ids(range, &self.map.nodes),
                    ))
                }
            }
        }
    });

    quote! {
        #vis struct #view<'a> {
            map: &'a #map,
        }

        impl<'a> #view<'a> {
            #immutable_category

            #vis fn iter(&self) -> #iter<'a> {
                #iter::new(#refs::new(
                    &self.map.nodes,
                    self.map.#index.iter_ids(&self.map.nodes),
                ))
            }

            #immutable_range
        }

        #vis struct #view_mut<'a> {
            map: &'a mut #map,
        }

        impl #view_mut<'_> {
            #mutable_read_category

            #vis fn iter(&self) -> #iter<'_> {
                #iter::new(#refs::new(
                    &self.map.nodes,
                    self.map.#index.iter_ids(&self.map.nodes),
                ))
            }

            #mutable_range
            #mutations
        }

        impl ::multi_index_map::IndexView for #view<'_> {
            type Value = #element;
            type Key = #ty;
            type Iter<'a> = #iter<'a> where Self: 'a;

            fn len(&self) -> usize {
                self.map.#index.len()
            }

            fn iter(&self) -> Self::Iter<'_> {
                #iter::new(#refs::new(
                    &self.map.nodes,
                    self.map.#index.iter_ids(&self.map.nodes),
                ))
            }
        }

        impl ::multi_index_map::IndexView for #view_mut<'_> {
            type Value = #element;
            type Key = #ty;
            type Iter<'a> = #iter<'a> where Self: 'a;

            fn len(&self) -> usize {
                self.map.#index.len()
            }

            fn iter(&self) -> Self::Iter<'_> {
                #iter::new(#refs::new(
                    &self.map.nodes,
                    self.map.#index.iter_ids(&self.map.nodes),
                ))
            }
        }

        #unique_traits
        #non_unique_traits
        #ordered_traits
    }
}
