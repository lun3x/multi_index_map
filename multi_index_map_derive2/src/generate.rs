use crate::model::{Index, Input};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

pub(crate) fn generate(mut input: Input) -> TokenStream {
    let names = Names::new(&input);
    let export_vis = input.vis.clone();
    input.rebase_for_child_module();
    let indexes = input
        .indexes
        .iter()
        .map(|index| IndexNames::new(&names, index))
        .collect::<Vec<_>>();
    let node = generate_node_and_specs(&input, &names, &indexes);
    let update = generate_update(&input, &names);
    let iterators = generate_iterators(&input, &names, &indexes);
    let map = generate_map(&input, &names, &indexes);
    let views = indexes
        .iter()
        .map(|index| generate_view(&input, &names, index));
    let compatibility = generate_compatibility(&input, &names, &indexes);
    let module = &names.module;
    let map_name = &names.map;

    quote! {
        #[doc(hidden)]
        #[allow(non_snake_case, unused_imports)]
        mod #module {
            use super::*;

            #node
            #update
            #iterators
            #map
            #(#views)*
            #compatibility
        }

        #[allow(unused_imports)]
        #export_vis use #module::#map_name;
    }
}

struct Names {
    element: Ident,
    module: Ident,
    map: Ident,
    inner: Ident,
    node: Ident,
    update: Ident,
    refs: Ident,
    selector: Ident,
}

impl Names {
    fn new(input: &Input) -> Self {
        let element = input.element.clone();
        let map = format_ident!("MultiIndex{}Map", element);
        Self {
            module: format_ident!("__multi_index_map2_{}", element),
            inner: format_ident!("__{}Inner", map),
            node: format_ident!("__{}Node", map),
            update: format_ident!("{}Update", map),
            refs: format_ident!("__{}Refs", map),
            selector: format_ident!("{}Index", map),
            element,
            map,
        }
    }
}

struct IndexNames<'a> {
    index: &'a Index,
    kind: Ident,
    spec: Ident,
    link: Ident,
    storage: Ident,
    view: Ident,
    view_mut: Ident,
    iter: Ident,
    equal: Ident,
    range: Ident,
}

impl<'a> IndexNames<'a> {
    fn new(names: &Names, index: &'a Index) -> Self {
        let map = &names.map;
        let n = index.ordinal;
        Self {
            index,
            kind: format_ident!("__{}Index{}Kind", map, n),
            spec: format_ident!("__{}Index{}Spec", map, n),
            link: format_ident!("__mim_index_{}_link", n),
            storage: format_ident!("__mim_index_{}", n),
            view: format_ident!("__{}Index{}View", map, n),
            view_mut: format_ident!("__{}Index{}ViewMut", map, n),
            iter: format_ident!("__{}Index{}Iter", map, n),
            equal: format_ident!("__{}Index{}EqualRange", map, n),
            range: format_ident!("__{}Index{}Range", map, n),
        }
    }

    fn accessor(&self) -> &syn::Path {
        &self.index.accessor
    }

    fn owned_key(&self) -> TokenStream {
        if let Some(field) = self.index.single_field() {
            let ty = &field.ty;
            quote!(#ty)
        } else {
            let types = self.index.fields.iter().map(|field| &field.ty);
            quote!((#(#types,)*))
        }
    }

    fn borrowed_key(&self) -> TokenStream {
        if let Some(field) = self.index.single_field() {
            let ty = &field.ty;
            quote!(&'a #ty)
        } else {
            let types = self.index.fields.iter().map(|field| &field.ty);
            quote!((#(&'a #types,)*))
        }
    }

    fn key_expr(&self) -> TokenStream {
        if let Some(field) = self.index.single_field() {
            let ident = &field.ident;
            quote!(&value.#ident)
        } else {
            let fields = self.index.fields.iter().map(|field| {
                let ident = &field.ident;
                quote!(&value.#ident)
            });
            quote!((#(#fields,)*))
        }
    }
}

fn generate_node_and_specs(
    input: &Input,
    names: &Names,
    indexes: &[IndexNames<'_>],
) -> TokenStream {
    let vis = input.child_visibility();
    let element = &names.element;
    let node = &names.node;
    let links = indexes.iter().map(|index| {
        let link = &index.link;
        let kind = &index.kind;
        quote!(#link: <#kind as ::multi_index_map::__private::IndexCategory>::Link)
    });
    let defaults = indexes.iter().map(|index| {
        let link = &index.link;
        quote!(#link: ::std::default::Default::default())
    });
    let specs = indexes.iter().map(|index| {
        let accessor = index.accessor();
        let kind = &index.kind;
        let spec = &index.spec;
        let link = &index.link;
        let borrowed_key = index.borrowed_key();
        let key_expr = index.key_expr();
        quote! {
            #vis type #kind = <#accessor as ::multi_index_map::MultiIndexAccessor>::Kind;
            #vis struct #spec;

            impl ::multi_index_map::__private::IndexSpec<#node> for #spec {
                type Key<'a> = #borrowed_key;
                type Link = <#kind as ::multi_index_map::__private::IndexCategory>::Link;
                const NAME: &'static str =
                    <#accessor as ::multi_index_map::MultiIndexAccessor>::NAME;

                fn key(value: &#element) -> Self::Key<'_> {
                    #key_expr
                }

                fn link(node: &#node) -> &Self::Link {
                    &node.#link
                }

                fn link_mut(node: &mut #node) -> &mut Self::Link {
                    &mut node.#link
                }
            }
        }
    });

    quote! {
        #vis struct #node {
            value: #element,
            #(#links,)*
        }

        impl #node {
            fn new(value: #element) -> Self {
                Self {
                    value,
                    #(#defaults,)*
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
    let vis = input.child_visibility();
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

fn generate_iterators(input: &Input, names: &Names, indexes: &[IndexNames<'_>]) -> TokenStream {
    let refs = &names.refs;
    let node = &names.node;
    let element = &names.element;
    let wrappers = indexes
        .iter()
        .map(|index| generate_index_iterators(input, names, index));
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

fn generate_index_iterators(input: &Input, names: &Names, index: &IndexNames<'_>) -> TokenStream {
    let vis = input.child_visibility();
    let element = &names.element;
    let refs = &names.refs;
    let node = &names.node;
    let accessor = index.accessor();
    let spec = &index.spec;
    let iter = &index.iter;
    let equal = &index.equal;
    let range = &index.range;

    quote! {
        #[doc(hidden)]
        #vis struct #iter<'a, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            inner: #refs<'a, <K as ::multi_index_map::__private::IndexKind<#node, #spec>>::Ids<'a>>,
        }

        impl<'a, K> Iterator for #iter<'a, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            type Item = &'a #element;
            fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
            fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
        }

        impl<K> DoubleEndedIterator for #iter<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::IndexKind<#node, #spec>,
            for<'a> <K as ::multi_index_map::__private::IndexKind<#node, #spec>>::Ids<'a>:
                DoubleEndedIterator,
        {
            fn next_back(&mut self) -> Option<Self::Item> { self.inner.next_back() }
        }

        #[doc(hidden)]
        #vis struct #equal<'a, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            inner: #refs<'a, <K as ::multi_index_map::__private::IndexKind<#node, #spec>>::EqualIds<'a>>,
        }

        impl<'a, K> Iterator for #equal<'a, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            type Item = &'a #element;
            fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
            fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
        }

        impl<K> DoubleEndedIterator for #equal<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::IndexKind<#node, #spec>,
            for<'a> <K as ::multi_index_map::__private::IndexKind<#node, #spec>>::EqualIds<'a>:
                DoubleEndedIterator,
        {
            fn next_back(&mut self) -> Option<Self::Item> { self.inner.next_back() }
        }

        #[doc(hidden)]
        #vis struct #range<'a, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::OrderedIndexKind<#node, #spec>,
        {
            inner: #refs<'a, <K as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::RangeIds<'a>>,
        }

        impl<'a, K> Iterator for #range<'a, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::OrderedIndexKind<#node, #spec>,
        {
            type Item = &'a #element;
            fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
            fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
        }

        impl<K> DoubleEndedIterator for #range<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::OrderedIndexKind<#node, #spec>,
        {
            fn next_back(&mut self) -> Option<Self::Item> { self.inner.next_back() }
        }
    }
}

fn generate_map(input: &Input, names: &Names, indexes: &[IndexNames<'_>]) -> TokenStream {
    let vis = input.child_visibility();
    let element = &names.element;
    let map = &names.map;
    let inner = &names.inner;
    let node = &names.node;
    let update = &names.update;
    let selector = &names.selector;

    let storages = indexes.iter().map(|index| {
        let storage = &index.storage;
        let kind = &index.kind;
        let spec = &index.spec;
        quote!(#storage: <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::Index)
    });
    let defaults = indexes.iter().map(|index| {
        let storage = &index.storage;
        quote!(#storage: ::std::default::Default::default())
    });
    let selector_impls = indexes.iter().map(|index| {
        let accessor = index.accessor();
        let kind = &index.kind;
        let view = &index.view;
        let view_mut = &index.view_mut;
        quote! {
            impl #selector for #accessor {
                type View<'a> = #view<'a, #kind>;
                type ViewMut<'a> = #view_mut<'a, #kind>;

                fn view(map: &#map) -> Self::View<'_> {
                    #view { map, marker: ::std::marker::PhantomData }
                }

                fn view_mut(map: &mut #map) -> Self::ViewMut<'_> {
                    #view_mut { map, marker: ::std::marker::PhantomData }
                }
            }
        }
    });
    let reserves = indexes.iter().map(|index| {
        let kind = &index.kind;
        let spec = &index.spec;
        let storage = &index.storage;
        quote! {
            <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::reserve_for_insert(
                &mut self.#storage,
                &mut self.nodes,
            );
        }
    });
    let inserts = indexes.iter().map(|index| {
        let accessor = index.accessor();
        let kind = &index.kind;
        let spec = &index.spec;
        let storage = &index.storage;
        quote! {
            if <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::insert(
                &mut self.#storage,
                id,
                &mut self.nodes,
            ).is_err() {
                self.unlink_all(id);
                return Some(<#accessor as ::multi_index_map::MultiIndexAccessor>::NAME);
            }
        }
    });
    let removes = indexes.iter().map(|index| {
        let kind = &index.kind;
        let spec = &index.spec;
        let storage = &index.storage;
        quote! {
            <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::remove(
                &mut self.#storage,
                id,
                &mut self.nodes,
            );
        }
    });
    let reconciles = indexes.iter().map(|index| {
        let accessor = index.accessor();
        let kind = &index.kind;
        let spec = &index.spec;
        let storage = &index.storage;
        quote! {
            if conflict.is_none()
                && <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::reconcile(
                    &mut self.#storage,
                    id,
                    &mut self.nodes,
                ).is_err()
            {
                conflict = Some(<#accessor as ::multi_index_map::MultiIndexAccessor>::NAME);
            }
        }
    });
    let validates = indexes.iter().map(|index| {
        let kind = &index.kind;
        let spec = &index.spec;
        let storage = &index.storage;
        quote! {
            <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::validate(
                &self.#storage,
                &self.nodes,
            )?;
        }
    });
    let lengths = indexes.iter().map(|index| {
        let kind = &index.kind;
        let spec = &index.spec;
        let storage = &index.storage;
        quote!(<#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::len(&self.#storage))
    });
    let update_expr = update_expr(input, names);
    let compatibility_helpers = generate_compatibility_helpers(input, names, indexes);

    quote! {
        #vis trait #selector: ::multi_index_map::MultiIndexAccessor {
            type View<'a> where Self: 'a;
            type ViewMut<'a> where Self: 'a;
            fn view(map: &#map) -> Self::View<'_>;
            fn view_mut(map: &mut #map) -> Self::ViewMut<'_>;
        }

        #vis struct #map {
            inner: #inner,
        }

        struct #inner {
            nodes: ::multi_index_map::__private::Slab<#node>,
            #(#storages,)*
        }

        impl Default for #inner {
            fn default() -> Self {
                Self {
                    nodes: ::std::default::Default::default(),
                    #(#defaults,)*
                }
            }
        }

        impl Default for #map {
            fn default() -> Self {
                Self {
                    inner: ::std::default::Default::default(),
                }
            }
        }

        impl #map {
            #vis fn new() -> Self { Self::default() }
            #vis fn len(&self) -> usize { self.inner.nodes.len() }
            #vis fn is_empty(&self) -> bool { self.inner.nodes.is_empty() }

            #vis fn by<I: #selector>(&self) -> I::View<'_> {
                I::view(self)
            }

            #vis fn by_mut<I: #selector>(&mut self) -> I::ViewMut<'_> {
                I::view_mut(self)
            }

            #vis fn try_insert(
                &mut self,
                value: #element,
            ) -> Result<&#element, ::multi_index_map::Conflict<#element>> {
                self.inner.try_insert(value)
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
                self.inner.clear();
            }

            #vis fn validate(&self) -> Result<(), String> {
                self.inner.validate()
            }
        }

        impl #inner {
            fn try_insert(
                &mut self,
                value: #element,
            ) -> Result<&#element, ::multi_index_map::Conflict<#element>> {
                self.reserve_all();
                let id = ::multi_index_map::__private::NodeId(self.nodes.insert(#node::new(value)));
                if let Some(index) = self.link_all(id) {
                    let value = self.nodes.remove(id.0).value;
                    self.validate_debug();
                    return Err(::multi_index_map::Conflict { index, value });
                }
                self.validate_debug();
                Ok(&self.nodes[id.0].value)
            }

            fn clear(&mut self) {
                let ids = self.nodes.iter()
                    .map(|(id, _)| ::multi_index_map::__private::NodeId(id))
                    .collect::<Vec<_>>();
                for id in ids {
                    self.remove_id(id);
                }
                self.validate_debug();
            }

            fn reserve_all(&mut self) {
                #(#reserves)*
            }

            fn link_all(&mut self, id: ::multi_index_map::__private::NodeId) -> Option<&'static str> {
                #(#inserts)*
                None
            }

            fn unlink_all(&mut self, id: ::multi_index_map::__private::NodeId) {
                #(#removes)*
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
                self.unlink_all(id);
                self.reserve_all();
                let candidate =
                    ::multi_index_map::__private::NodeId(self.nodes.insert(#node::new(replacement)));
                if let Some(index) = self.link_all(candidate) {
                    let replacement = self.nodes.remove(candidate.0).value;
                    assert!(self.link_all(id).is_none(), "restoring replaced element conflicted");
                    self.validate_debug();
                    return Err(::multi_index_map::Conflict { index, value: replacement });
                }
                self.unlink_all(candidate);
                let replacement = self.nodes.remove(candidate.0).value;
                let old = ::std::mem::replace(&mut self.nodes[id.0].value, replacement);
                assert!(self.link_all(id).is_none(), "prepared replacement unexpectedly conflicted");
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
                    if !self.nodes.contains(id.0) { continue; }
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
                f(#update_expr);
                &self.nodes[id.0].value
            }

            fn update_ids(
                &mut self,
                ids: Vec<::multi_index_map::__private::NodeId>,
                mut f: impl FnMut(#update<'_>),
            ) -> usize {
                for id in &ids {
                    let value = &mut self.nodes[id.0].value;
                    f(#update_expr);
                }
                ids.len()
            }

            #compatibility_helpers

            fn validate(&self) -> Result<(), String> {
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

        #(#selector_impls)*
    }
}

fn update_expr(input: &Input, names: &Names) -> TokenStream {
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

fn compatibility_indexes<'a>(
    input: &Input,
    indexes: &'a [IndexNames<'a>],
) -> Vec<&'a IndexNames<'a>> {
    indexes
        .iter()
        .filter(|index| {
            let Some(field) = index.index.single_field() else {
                return false;
            };
            input
                .indexes
                .iter()
                .filter(|candidate| {
                    candidate
                        .single_field()
                        .is_some_and(|candidate| candidate.ident == field.ident)
                })
                .count()
                == 1
        })
        .collect()
}

fn update_tuple_type(input: &Input) -> TokenStream {
    let types = input.unindexed.iter().map(|field| {
        let ty = &field.ty;
        quote!(&mut #ty)
    });
    quote!((#(#types,)*))
}

fn generate_compatibility_helpers(
    input: &Input,
    names: &Names,
    indexes: &[IndexNames<'_>],
) -> TokenStream {
    if compatibility_indexes(input, indexes).is_empty() {
        return TokenStream::new();
    }
    let element = &names.element;
    let tuple_type = update_tuple_type(input);
    let update_fields = if input.unindexed.is_empty() {
        quote! {
            fn update_fields_for_ids(
                &mut self,
                mut ids: Vec<::multi_index_map::__private::NodeId>,
            ) -> Vec<#tuple_type> {
                ids.sort_unstable_by_key(|id| id.0);
                ids.dedup();
                vec![(); ids.len()]
            }
        }
    } else {
        let values = input.unindexed.iter().map(|field| {
            let ident = &field.ident;
            quote!(&mut node.value.#ident)
        });
        quote! {
            fn update_fields_for_ids(
                &mut self,
                mut ids: Vec<::multi_index_map::__private::NodeId>,
            ) -> Vec<#tuple_type> {
                ids.sort_unstable_by_key(|id| id.0);
                assert!(!ids.windows(2).any(|pair| pair[0] == pair[1]));
                let mut fields = Vec::with_capacity(ids.len());
                let mut targets = ids.into_iter();
                let mut target = targets.next();
                for (slot, node) in self.nodes.iter_mut() {
                    if target.map(|id| id.0) == Some(slot) {
                        fields.push((#(#values,)*));
                        target = targets.next();
                    }
                }
                assert!(target.is_none(), "compatibility accessor targeted a missing node");
                fields
            }
        }
    };
    quote! {
        #update_fields

        fn order_refs_for_ids(
            &self,
            ids: &[::multi_index_map::__private::NodeId],
        ) -> Vec<&#element> {
            ids.iter().filter_map(|id| self.nodes.get(id.0).map(|node| &node.value)).collect()
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
    }
}

struct Query {
    generics: TokenStream,
    argument: TokenStream,
    query_type: TokenStream,
    key_ref: TokenStream,
    ordered_bounds: TokenStream,
}

fn query(index: &Index) -> Query {
    if let Some(field) = index.single_field() {
        let ty = &field.ty;
        Query {
            generics: quote!(Q: ?Sized),
            argument: quote!(key: &Q),
            query_type: quote!(Q),
            key_ref: quote!(key),
            ordered_bounds: quote!(
                #ty: ::std::borrow::Borrow<Q>,
                Q: Ord,
            ),
        }
    } else {
        let q = (0..index.fields.len())
            .map(|n| format_ident!("Q{}", n))
            .collect::<Vec<_>>();
        let types = index
            .fields
            .iter()
            .map(|field| &field.ty)
            .collect::<Vec<_>>();
        Query {
            generics: quote!('query, #(#q: ?Sized + 'query),*),
            argument: quote!(key: (#(&'query #q,)*)),
            query_type: quote!((#(&'query #q,)*)),
            key_ref: quote!(&key),
            ordered_bounds: quote!(
                #(#types: ::std::borrow::Borrow<#q>, #q: Ord,)*
            ),
        }
    }
}

fn generate_view(input: &Input, names: &Names, index: &IndexNames<'_>) -> TokenStream {
    let vis = input.child_visibility();
    let element = &names.element;
    let map = &names.map;
    let node = &names.node;
    let update = &names.update;
    let refs = &names.refs;
    let accessor = index.accessor();
    let spec = &index.spec;
    let storage = &index.storage;
    let view = &index.view;
    let view_mut = &index.view_mut;
    let iter = &index.iter;
    let equal = &index.equal;
    let range = &index.range;
    let owned_key = index.owned_key();
    let query = query(index.index);
    let q_generics = &query.generics;
    let argument = &query.argument;
    let q_ty = &query.query_type;
    let key_ref = &query.key_ref;
    let ordered_bounds = &query.ordered_bounds;
    let capabilities = generate_capability_traits(input, names, index);

    quote! {
        #[doc(hidden)]
        #vis struct #view<'a, K> {
            map: &'a #map,
            marker: ::std::marker::PhantomData<K>,
        }

        #[doc(hidden)]
        #vis struct #view_mut<'a, K> {
            map: &'a mut #map,
            marker: ::std::marker::PhantomData<K>,
        }

        impl<'a, K> #view<'a, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            #vis fn iter(&self) -> #iter<'a, K> {
                #iter {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <K as ::multi_index_map::__private::IndexKind<#node, #spec>>::iter_ids(
                            &self.map.inner.#storage,
                            &self.map.inner.nodes,
                        ),
                    ),
                }
            }
        }

        impl<K> #view_mut<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            #vis fn iter(&self) -> #iter<'_, K> {
                #iter {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <K as ::multi_index_map::__private::IndexKind<#node, #spec>>::iter_ids(
                            &self.map.inner.#storage,
                            &self.map.inner.nodes,
                        ),
                    ),
                }
            }
        }

        impl<'a, K> #view<'a, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::UniqueCategory
                + ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            #vis fn get<#q_generics>(&self, #argument) -> Option<&'a #element>
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                ).map(|id| &self.map.inner.nodes[id.0].value)
            }

            #vis fn contains_key<#q_generics>(&self, #argument) -> bool
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                ).is_some()
            }
        }

        impl<K> #view_mut<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::UniqueCategory
                + ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            #vis fn get<#q_generics>(&self, #argument) -> Option<&#element>
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                ).map(|id| &self.map.inner.nodes[id.0].value)
            }

            #vis fn contains_key<#q_generics>(&self, #argument) -> bool
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                ).is_some()
            }

            #vis fn remove<#q_generics>(&mut self, #argument) -> Option<#element>
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let id = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                )?;
                let value = self.map.inner.remove_id(id);
                self.map.inner.validate_debug();
                Some(value)
            }

            #vis fn replace<#q_generics>(
                &mut self,
                #argument,
                replacement: #element,
            ) -> Result<Option<#element>, ::multi_index_map::Conflict<#element>>
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let Some(id) = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                ) else { return Ok(None); };
                self.map.inner.replace_id(id, replacement).map(Some)
            }

            #vis fn modify<#q_generics>(
                &mut self,
                #argument,
                f: impl FnOnce(&mut #element),
            ) -> Result<Option<&#element>, ::multi_index_map::Conflict<#element>>
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let Some(id) = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                ) else { return Ok(None); };
                self.map.inner.modify_id(id, f).map(Some)
            }

            #vis fn update<#q_generics>(
                &mut self,
                #argument,
                f: impl FnOnce(#update<'_>),
            ) -> Option<&#element>
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let id = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                )?;
                Some(self.map.inner.update_id(id, f))
            }
        }

        impl<'a, K> #view<'a, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::NonUniqueCategory
                + ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            #vis fn equal_range<#q_generics>(&self, #argument) -> #equal<'a, K>
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                #equal {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::equal_iter_ids(
                            &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl<K> #view_mut<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::NonUniqueCategory
                + ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            #vis fn equal_range<#q_generics>(&self, #argument) -> #equal<'_, K>
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                #equal {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::equal_iter_ids(
                            &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                        ),
                    ),
                }
            }

            #vis fn remove_all<#q_generics>(&mut self, #argument) -> Vec<#element>
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let ids = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::equal_ids(
                    &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                );
                let values = ids.into_iter().map(|id| self.map.inner.remove_id(id)).collect();
                self.map.inner.validate_debug();
                values
            }

            #vis fn modify_all<#q_generics>(
                &mut self,
                #argument,
                f: impl FnMut(&mut #element),
            ) -> ::multi_index_map::ModifyAllResult<#element>
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let ids = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::equal_ids(
                    &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                );
                self.map.inner.modify_ids(ids, f)
            }

            #vis fn update_all<#q_generics>(
                &mut self,
                #argument,
                f: impl FnMut(#update<'_>),
            ) -> usize
            where
                K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let ids = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::equal_ids(
                    &self.map.inner.#storage, #key_ref, &self.map.inner.nodes
                );
                self.map.inner.update_ids(ids, f)
            }
        }

        impl<'a, K> #view<'a, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::OrderedCategory
                + ::multi_index_map::__private::OrderedIndexKind<#node, #spec>,
        {
            #vis fn range<#q_generics, R>(&self, range_value: R) -> #range<'a, K>
            where
                R: ::std::ops::RangeBounds<#q_ty>,
                #ordered_bounds
                for<'key> <#spec as ::multi_index_map::__private::IndexSpec<#node>>::Key<'key>:
                    ::multi_index_map::__private::Compare<#q_ty>,
            {
                #range {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <K as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::range_iter_ids(
                            &self.map.inner.#storage, range_value, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl<K> #view_mut<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::OrderedCategory
                + ::multi_index_map::__private::OrderedIndexKind<#node, #spec>,
        {
            #vis fn range<#q_generics, R>(&self, range_value: R) -> #range<'_, K>
            where
                R: ::std::ops::RangeBounds<#q_ty>,
                #ordered_bounds
                for<'key> <#spec as ::multi_index_map::__private::IndexSpec<#node>>::Key<'key>:
                    ::multi_index_map::__private::Compare<#q_ty>,
            {
                #range {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <K as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::range_iter_ids(
                            &self.map.inner.#storage, range_value, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl<K> ::multi_index_map::IndexView for #view<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            type Value = #element;
            type Key = #owned_key;
            type Iter<'a> = #iter<'a, K> where Self: 'a;

            fn len(&self) -> usize {
                <K as ::multi_index_map::__private::IndexKind<#node, #spec>>::len(&self.map.inner.#storage)
            }

            fn iter(&self) -> Self::Iter<'_> { #view::iter(self) }
        }

        impl<K> ::multi_index_map::IndexView for #view_mut<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::IndexKind<#node, #spec>,
        {
            type Value = #element;
            type Key = #owned_key;
            type Iter<'a> = #iter<'a, K> where Self: 'a;

            fn len(&self) -> usize {
                <K as ::multi_index_map::__private::IndexKind<#node, #spec>>::len(&self.map.inner.#storage)
            }

            fn iter(&self) -> Self::Iter<'_> { #view_mut::iter(self) }
        }

        #capabilities
    }
}

fn generate_capability_traits(
    _input: &Input,
    names: &Names,
    index: &IndexNames<'_>,
) -> TokenStream {
    let element = &names.element;
    let node = &names.node;
    let update = &names.update;
    let refs = &names.refs;
    let accessor = index.accessor();
    let spec = &index.spec;
    let storage = &index.storage;
    let view = &index.view;
    let view_mut = &index.view_mut;
    let equal = &index.equal;
    let range = &index.range;
    let (query_ty, query_bound, query_setup, query_ref, ordered_key_bounds, range_value) =
        if let Some(field) = index.index.single_field() {
            let ty = &field.ty;
            (
                quote!(#ty),
                quote!(K: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #ty>),
                TokenStream::new(),
                quote!(key),
                quote!(
                    #ty: Ord,
                    for<'key> <#spec as ::multi_index_map::__private::IndexSpec<#node>>::Key<'key>:
                        ::multi_index_map::__private::Compare<#ty>,
                ),
                quote!(range_value),
            )
        } else {
            let types = index
                .index
                .fields
                .iter()
                .map(|field| &field.ty)
                .collect::<Vec<_>>();
            let positions = (0..types.len()).map(syn::Index::from).collect::<Vec<_>>();
            let query_bound = quote!(
                for<'query> K: ::multi_index_map::__private::QueryIndexKind<
                    #node,
                    #spec,
                    (#(&'query #types,)*)
                >
            );
            let ordered_key_bounds = quote!(
                #(#types: Ord,)*
                for<'key, 'query>
                    <#spec as ::multi_index_map::__private::IndexSpec<#node>>::Key<'key>:
                        ::multi_index_map::__private::Compare<(#(&'query #types,)*)>,
            );
            let query_setup = quote!(let query = (#(&key.#positions,)*););
            let query_ref = quote!(&query);
            let range_value = quote!({
                let start = match range_value.start_bound() {
                    ::std::ops::Bound::Included(key) =>
                        ::std::ops::Bound::Included((#(&key.#positions,)*)),
                    ::std::ops::Bound::Excluded(key) =>
                        ::std::ops::Bound::Excluded((#(&key.#positions,)*)),
                    ::std::ops::Bound::Unbounded => ::std::ops::Bound::Unbounded,
                };
                let end = match range_value.end_bound() {
                    ::std::ops::Bound::Included(key) =>
                        ::std::ops::Bound::Included((#(&key.#positions,)*)),
                    ::std::ops::Bound::Excluded(key) =>
                        ::std::ops::Bound::Excluded((#(&key.#positions,)*)),
                    ::std::ops::Bound::Unbounded => ::std::ops::Bound::Unbounded,
                };
                ::multi_index_map::__private::QueryRange::new(start, end)
            });
            (
                quote!((#(&'_ #types,)*)),
                query_bound,
                query_setup,
                query_ref,
                ordered_key_bounds,
                range_value,
            )
        };

    quote! {
        impl<K> ::multi_index_map::UniqueView for #view<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::UniqueCategory
                + ::multi_index_map::__private::IndexKind<#node, #spec>,
            #query_bound,
        {
            fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
                #query_setup
                <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    &self.map.inner.#storage, #query_ref, &self.map.inner.nodes
                ).map(|id| &self.map.inner.nodes[id.0].value)
            }
        }

        impl<K> ::multi_index_map::UniqueView for #view_mut<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::UniqueCategory
                + ::multi_index_map::__private::IndexKind<#node, #spec>,
            #query_bound,
        {
            fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
                #query_setup
                <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    &self.map.inner.#storage, #query_ref, &self.map.inner.nodes
                ).map(|id| &self.map.inner.nodes[id.0].value)
            }
        }

        impl<K> ::multi_index_map::UniqueViewMut for #view_mut<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::UniqueCategory
                + ::multi_index_map::__private::IndexKind<#node, #spec>,
            #query_bound,
        {
            type Conflict = ::multi_index_map::Conflict<#element>;
            type Update<'a> = #update<'a>;

            fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
                #query_setup
                let id = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    &self.map.inner.#storage, #query_ref, &self.map.inner.nodes
                )?;
                let value = self.map.inner.remove_id(id);
                self.map.inner.validate_debug();
                Some(value)
            }

            fn replace(
                &mut self,
                key: &Self::Key,
                replacement: Self::Value,
            ) -> Result<Option<Self::Value>, Self::Conflict> {
                #query_setup
                let Some(id) = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    &self.map.inner.#storage, #query_ref, &self.map.inner.nodes
                ) else { return Ok(None); };
                self.map.inner.replace_id(id, replacement).map(Some)
            }

            fn modify<F>(
                &mut self,
                key: &Self::Key,
                f: F,
            ) -> Result<Option<&Self::Value>, Self::Conflict>
            where
                F: FnOnce(&mut Self::Value),
            {
                #query_setup
                let Some(id) = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    &self.map.inner.#storage, #query_ref, &self.map.inner.nodes
                ) else { return Ok(None); };
                self.map.inner.modify_id(id, f).map(Some)
            }

            fn update<F>(&mut self, key: &Self::Key, f: F) -> Option<&Self::Value>
            where
                F: for<'a> FnOnce(Self::Update<'a>),
            {
                #query_setup
                let id = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    &self.map.inner.#storage, #query_ref, &self.map.inner.nodes
                )?;
                Some(self.map.inner.update_id(id, f))
            }
        }

        impl<K> ::multi_index_map::NonUniqueView for #view<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::NonUniqueCategory
                + ::multi_index_map::__private::IndexKind<#node, #spec>,
            #query_bound,
        {
            type EqualRange<'a> = #equal<'a, K> where Self: 'a;

            fn equal_range(&self, key: &Self::Key) -> Self::EqualRange<'_> {
                #query_setup
                #equal {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::equal_iter_ids(
                            &self.map.inner.#storage, #query_ref, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl<K> ::multi_index_map::NonUniqueView for #view_mut<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::NonUniqueCategory
                + ::multi_index_map::__private::IndexKind<#node, #spec>,
            #query_bound,
        {
            type EqualRange<'a> = #equal<'a, K> where Self: 'a;

            fn equal_range(&self, key: &Self::Key) -> Self::EqualRange<'_> {
                #query_setup
                #equal {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::equal_iter_ids(
                            &self.map.inner.#storage, #query_ref, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl<K> ::multi_index_map::NonUniqueViewMut for #view_mut<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::NonUniqueCategory
                + ::multi_index_map::__private::IndexKind<#node, #spec>,
            #query_bound,
        {
            type ModifyAllResult = ::multi_index_map::ModifyAllResult<#element>;
            type Update<'a> = #update<'a>;

            fn remove_all(&mut self, key: &Self::Key) -> Vec<Self::Value> {
                #query_setup
                let ids = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::equal_ids(
                    &self.map.inner.#storage, #query_ref, &self.map.inner.nodes
                );
                let values = ids.into_iter().map(|id| self.map.inner.remove_id(id)).collect();
                self.map.inner.validate_debug();
                values
            }

            fn modify_all<F>(&mut self, key: &Self::Key, f: F) -> Self::ModifyAllResult
            where
                F: FnMut(&mut Self::Value),
            {
                #query_setup
                let ids = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::equal_ids(
                    &self.map.inner.#storage, #query_ref, &self.map.inner.nodes
                );
                self.map.inner.modify_ids(ids, f)
            }

            fn update_all<F>(&mut self, key: &Self::Key, f: F) -> usize
            where
                F: for<'a> FnMut(Self::Update<'a>),
            {
                #query_setup
                let ids = <K as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::equal_ids(
                    &self.map.inner.#storage, #query_ref, &self.map.inner.nodes
                );
                self.map.inner.update_ids(ids, f)
            }
        }

        impl<K> ::multi_index_map::OrderedView for #view<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::OrderedCategory
                + ::multi_index_map::__private::OrderedIndexKind<#node, #spec>,
            #ordered_key_bounds
        {
            type Range<'a> = #range<'a, K> where Self: 'a;

            fn range<R>(&self, range_value: R) -> Self::Range<'_>
            where
                R: ::std::ops::RangeBounds<Self::Key>,
            {
                #range {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <K as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::range_iter_ids(
                            &self.map.inner.#storage, #range_value, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl<K> ::multi_index_map::OrderedView for #view_mut<'_, K>
        where
            #accessor: ::multi_index_map::MultiIndexAccessor<Kind = K>,
            K: ::multi_index_map::__private::OrderedCategory
                + ::multi_index_map::__private::OrderedIndexKind<#node, #spec>,
            #ordered_key_bounds
        {
            type Range<'a> = #range<'a, K> where Self: 'a;

            fn range<R>(&self, range_value: R) -> Self::Range<'_>
            where
                R: ::std::ops::RangeBounds<Self::Key>,
            {
                #range {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <K as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::range_iter_ids(
                            &self.map.inner.#storage, #range_value, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }
    }
}

fn generate_compatibility(input: &Input, names: &Names, indexes: &[IndexNames<'_>]) -> TokenStream {
    let wrappers = compatibility_indexes(input, indexes)
        .into_iter()
        .map(|index| generate_inherent_compatibility_index(input, names, index));
    quote!(#(#wrappers)*)
}

fn generate_inherent_compatibility_index(
    input: &Input,
    names: &Names,
    index: &IndexNames<'_>,
) -> TokenStream {
    let element = &names.element;
    let map = &names.map;
    let inner = &names.inner;
    let node = &names.node;
    let refs = &names.refs;
    let kind = &index.kind;
    let spec = &index.spec;
    let storage = &index.storage;
    let iter = &index.iter;
    let field = index.index.single_field().expect("filtered single index");
    let field_ident = &field.ident;
    let field_vis = &field.vis;
    let ty = &field.ty;
    let get_by = format_ident!("get_by_{}", field_ident);
    let get_mut_by = format_ident!("get_mut_by_{}", field_ident);
    let modify_by = format_ident!("modify_by_{}", field_ident);
    let update_by = format_ident!("update_by_{}", field_ident);
    let remove_by = format_ident!("remove_by_{}", field_ident);
    let iter_by = format_ident!("iter_by_{}", field_ident);
    let tuple_type = update_tuple_type(input);
    let collection = quote!(
        <#kind as ::multi_index_map::__private::CompatibilityKind>::Collection
    );
    let update_types = input.unindexed.iter().map(|field| {
        let ty = &field.ty;
        quote!(&mut #ty)
    });
    let update_types = update_types.collect::<Vec<_>>();
    let update_args = input.unindexed.iter().map(|field| {
        let ident = &field.ident;
        quote!(fields.#ident)
    });
    let update_args = update_args.collect::<Vec<_>>();

    quote! {
        impl #map {
            #[deprecated(note = "use map.by::<Accessor>().get/equal_range(key)")]
            #field_vis fn #get_by<Q: ?Sized>(&self, key: &Q) -> #collection<&#element>
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, Q>,
            {
                let values =
                    <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, Q>>::equal_ids(
                        &self.inner.#storage, key, &self.inner.nodes
                    )
                    .into_iter()
                    .map(|id| &self.inner.nodes[id.0].value)
                    .collect();
                <#kind as ::multi_index_map::__private::CompatibilityKind>::from_vec(values)
            }

            #[deprecated(note = "use map.by_mut::<Accessor>().update/update_all(key, ...)")]
            #field_vis fn #get_mut_by(&mut self, key: &#ty) -> #collection<#tuple_type> {
                let ids =
                    <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #ty>>::equal_ids(
                        &self.inner.#storage, key, &self.inner.nodes
                    );
                let fields = self.inner.update_fields_for_ids(ids);
                <#kind as ::multi_index_map::__private::CompatibilityKind>::from_vec(fields)
            }

            #[deprecated(note = "use map.by_mut::<Accessor>().modify/modify_all(key, ...)")]
            #field_vis fn #modify_by(
                &mut self,
                key: &#ty,
                f: impl FnMut(&mut #element),
            ) -> #collection<&#element> {
                let ids =
                    <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #ty>>::equal_ids(
                        &self.inner.#storage, key, &self.inner.nodes
                    );
                let result = self.inner.modify_ids(ids.clone(), f);
                #inner::panic_on_modify_conflicts(result);
                let values = self.inner.order_refs_for_ids(&ids);
                <#kind as ::multi_index_map::__private::CompatibilityKind>::from_vec(values)
            }

            #[deprecated(note = "use map.by_mut::<Accessor>().update/update_all(key, ...)")]
            #field_vis fn #update_by<Q: ?Sized>(
                &mut self,
                key: &Q,
                mut f: impl FnMut(#(#update_types),*),
            ) -> #collection<&#element>
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, Q>,
            {
                let ids =
                    <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, Q>>::equal_ids(
                        &self.inner.#storage, key, &self.inner.nodes
                    );
                for id in &ids {
                    self.inner.update_id(*id, |fields| f(#(#update_args),*));
                }
                let values = self.inner.order_refs_for_ids(&ids);
                <#kind as ::multi_index_map::__private::CompatibilityKind>::from_vec(values)
            }

            #[deprecated(note = "use map.by_mut::<Accessor>().remove/remove_all(key)")]
            #field_vis fn #remove_by(&mut self, key: &#ty) -> #collection<#element> {
                let ids =
                    <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #ty>>::equal_ids(
                        &self.inner.#storage, key, &self.inner.nodes
                    );
                let values = ids.into_iter().map(|id| self.inner.remove_id(id)).collect();
                self.inner.validate_debug();
                <#kind as ::multi_index_map::__private::CompatibilityKind>::from_vec(values)
            }

            #[deprecated(note = "use map.by::<Accessor>().iter()")]
            #field_vis fn #iter_by(&self) -> #iter<'_, #kind> {
                #iter {
                    inner: #refs::new(
                        &self.inner.nodes,
                        <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::iter_ids(
                            &self.inner.#storage, &self.inner.nodes
                        ),
                    ),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::generate;
    use crate::model::Input;
    use quote::ToTokens;
    use syn::{parse_quote, Fields, ImplItem, Item};

    #[test]
    fn top_level_contains_only_private_module_and_map_reexport() {
        let input = Input::parse(parse_quote! {
            pub struct Record {
                #[multi_index(ById)]
                pub id: u64,
                value: String,
            }
        })
        .unwrap();
        let file = syn::parse2::<syn::File>(generate(input)).unwrap();
        assert_eq!(file.items.len(), 2);
        assert!(
            matches!(&file.items[0], Item::Mod(module) if module.ident == "__multi_index_map2_Record")
        );
        assert!(matches!(&file.items[1], Item::Use(_)));

        let module = match &file.items[0] {
            Item::Mod(module) => module.content.as_ref().unwrap(),
            _ => unreachable!(),
        };
        let map = module
            .1
            .iter()
            .find_map(|item| match item {
                Item::Struct(item) if item.ident == "MultiIndexRecordMap" => Some(item),
                _ => None,
            })
            .unwrap();
        let Fields::Named(fields) = &map.fields else {
            panic!("generated map must have named fields");
        };
        assert_eq!(fields.named.len(), 1);
        assert_eq!(fields.named[0].ident.as_ref().unwrap(), "inner");

        let map_methods = module
            .1
            .iter()
            .filter_map(|item| match item {
                Item::Impl(item)
                    if item.self_ty.as_ref().to_token_stream().to_string()
                        == "MultiIndexRecordMap" =>
                {
                    Some(item)
                }
                _ => None,
            })
            .flat_map(|item| &item.items)
            .filter_map(|item| match item {
                ImplItem::Fn(method) => Some(method.sig.ident.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>();

        for private_method in [
            "link_all",
            "unlink_all",
            "remove_id",
            "replace_id",
            "modify_id",
            "update_id",
            "validate_debug",
        ] {
            assert!(!map_methods.iter().any(|method| method == private_method));
        }
    }
}
