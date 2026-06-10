use crate::model::{Index, Input};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::{
    parse_quote, GenericParam, Generics, Lifetime, LifetimeParam, TypeParam, WherePredicate,
};

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
        #[allow(non_camel_case_types, non_snake_case, non_upper_case_globals, unused_imports)]
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
    kind: Ident,
    query: Ident,
    range_param: Ident,
    iter_param: Ident,
    node_param: Ident,
    selector_param: Ident,
    query_components: Vec<Ident>,
    view_lifetime: Lifetime,
    update_lifetime: Lifetime,
    key_lifetime: Lifetime,
    query_lifetime: Lifetime,
}

impl Names {
    fn new(input: &Input) -> Self {
        let mut fresh = FreshNames::new(&input.generics);
        let element = input.element.clone();
        let map = format_ident!("MultiIndex{}Map", element);
        Self {
            module: format_ident!("__multi_index_map2_{}", element),
            inner: format_ident!("__{}Inner", map),
            node: format_ident!("__{}Node", map),
            update: format_ident!("{}Update", map),
            refs: format_ident!("__{}Refs", map),
            selector: format_ident!("{}Index", map),
            kind: fresh.ident("__MimKind"),
            query: fresh.ident("__MimQuery"),
            range_param: fresh.ident("__MimRange"),
            iter_param: fresh.ident("__MimIter"),
            node_param: fresh.ident("__MimNode"),
            selector_param: fresh.ident("__MimSelector"),
            query_components: (0..12)
                .map(|n| fresh.ident(&format!("__MimQuery{n}")))
                .collect(),
            view_lifetime: fresh.lifetime("__mim_view"),
            update_lifetime: fresh.lifetime("__mim_update"),
            key_lifetime: fresh.lifetime("__mim_key"),
            query_lifetime: fresh.lifetime("__mim_query"),
            element,
            map,
        }
    }
}

struct FreshNames {
    idents: HashSet<String>,
    lifetimes: HashSet<String>,
}

impl FreshNames {
    fn new(generics: &Generics) -> Self {
        let mut idents = HashSet::new();
        let mut lifetimes = HashSet::new();
        for param in &generics.params {
            match param {
                GenericParam::Lifetime(param) => {
                    lifetimes.insert(param.lifetime.ident.to_string());
                }
                GenericParam::Type(param) => {
                    idents.insert(param.ident.to_string());
                }
                GenericParam::Const(param) => {
                    idents.insert(param.ident.to_string());
                }
            }
        }
        Self { idents, lifetimes }
    }

    fn ident(&mut self, base: &str) -> Ident {
        for suffix in 0.. {
            let candidate = if suffix == 0 {
                base.to_string()
            } else {
                format!("{base}{suffix}")
            };
            if self.idents.insert(candidate.clone()) {
                return Ident::new(&candidate, Span::call_site());
            }
        }
        unreachable!()
    }

    fn lifetime(&mut self, base: &str) -> Lifetime {
        for suffix in 0.. {
            let candidate = if suffix == 0 {
                base.to_string()
            } else {
                format!("{base}{suffix}")
            };
            if self.lifetimes.insert(candidate.clone()) {
                return Lifetime::new(&format!("'{candidate}"), Span::call_site());
            }
        }
        unreachable!()
    }
}

fn helper_generics(input: &Input) -> Generics {
    let mut generics = input.generics.clone();
    for param in &mut generics.params {
        match param {
            GenericParam::Type(param) => param.default = None,
            GenericParam::Const(param) => param.default = None,
            GenericParam::Lifetime(_) => {}
        }
    }
    generics
}

fn with_lifetime(mut generics: Generics, lifetime: &Lifetime) -> Generics {
    generics.params.insert(
        0,
        GenericParam::Lifetime(LifetimeParam::new(lifetime.clone())),
    );
    generics
}

fn with_type(mut generics: Generics, ident: &Ident) -> Generics {
    generics
        .params
        .push(GenericParam::Type(TypeParam::from(ident.clone())));
    generics
}

fn with_predicates(
    mut generics: Generics,
    predicates: impl IntoIterator<Item = WherePredicate>,
) -> Generics {
    generics.make_where_clause().predicates.extend(predicates);
    generics
}

fn type_args(input: &Input) -> TokenStream {
    let generics = helper_generics(input);
    let (_, ty_generics, _) = generics.split_for_impl();
    quote!(#ty_generics)
}

fn argument_tokens(input: &Input) -> Vec<TokenStream> {
    input
        .generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Lifetime(param) => {
                let lifetime = &param.lifetime;
                quote!(#lifetime)
            }
            GenericParam::Type(param) => {
                let ident = &param.ident;
                quote!(#ident)
            }
            GenericParam::Const(param) => {
                let ident = &param.ident;
                quote!(#ident)
            }
        })
        .collect()
}

fn application(
    name: &Ident,
    input: &Input,
    prefix: impl IntoIterator<Item = TokenStream>,
    suffix: impl IntoIterator<Item = TokenStream>,
) -> TokenStream {
    let args = prefix
        .into_iter()
        .chain(argument_tokens(input))
        .chain(suffix)
        .collect::<Vec<_>>();
    if args.is_empty() {
        quote!(#name)
    } else {
        quote!(#name<#(#args),*>)
    }
}

fn applied(name: &Ident, input: &Input) -> TokenStream {
    application(name, input, [], [])
}

fn engine_predicates(
    input: &Input,
    names: &Names,
    indexes: &[IndexNames<'_>],
) -> Vec<WherePredicate> {
    let node = applied(&names.node, input);
    indexes
        .iter()
        .map(|index| {
            let kind = &index.kind;
            let spec = &index.spec;
            parse_quote!(#kind: ::multi_index_map::__private::IndexKind<#node, #spec>)
        })
        .chain(
            input
                .indexes
                .iter()
                .flat_map(|index| &index.fields)
                .map(|field| {
                    let ty = &field.ty;
                    parse_quote!(#ty: 'static)
                }),
        )
        .collect()
}

fn selected_index_predicates(
    selected_kind: &Ident,
    concrete_kind: &Ident,
    node: &TokenStream,
    spec: &Ident,
    inner: &TokenStream,
    binding: &TokenStream,
) -> Vec<WherePredicate> {
    vec![
        parse_quote!(
            #selected_kind: ::multi_index_map::__private::IndexCategory<
                Link = <#concrete_kind as ::multi_index_map::__private::IndexCategory>::Link
            >
        ),
        parse_quote!(#selected_kind: ::multi_index_map::__private::IndexKind<#node, #spec>),
        parse_quote!(#inner: #binding),
    ]
}

fn map_generics(
    input: &Input,
    names: &Names,
    indexes: &[IndexNames<'_>],
    defaults: bool,
) -> Generics {
    let generics = if defaults {
        input.generics.clone()
    } else {
        helper_generics(input)
    };
    with_predicates(generics, engine_predicates(input, names, indexes))
}

struct IndexNames<'a> {
    index: &'a Index,
    kind: Ident,
    spec: Ident,
    binding: Ident,
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
            binding: format_ident!("__{}Index{}Binding", map, n),
            link: format_ident!("__mim_index_{}_link", n),
            storage: format_ident!("__mim_index_{}", n),
            view: format_ident!("__{}Index{}View", map, n),
            view_mut: format_ident!("__{}Index{}ViewMut", map, n),
            iter: format_ident!("__{}Index{}Iter", map, n),
            equal: format_ident!("__{}Index{}EqualRange", map, n),
            range: format_ident!("__{}Index{}Range", map, n),
        }
    }

    fn selector(&self) -> &syn::Path {
        &self.index.selector
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

    fn borrowed_key(&self, lifetime: &Lifetime) -> TokenStream {
        if let Some(field) = self.index.single_field() {
            let ty = &field.ty;
            quote!(&#lifetime #ty)
        } else {
            let types = self.index.fields.iter().map(|field| &field.ty);
            quote!((#(&#lifetime #types,)*))
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
    let element = applied(&names.element, input);
    let node = &names.node;
    let node_ty = applied(node, input);
    let type_args = type_args(input);
    let node_generics = helper_generics(input);
    let (impl_generics, _, where_clause) = node_generics.split_for_impl();
    let key_lifetime = &names.key_lifetime;
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
        let selector = index.selector();
        let kind = &index.kind;
        let spec = &index.spec;
        let link = &index.link;
        let borrowed_key = index.borrowed_key(key_lifetime);
        let key_expr = index.key_expr();
        let spec_generics = with_predicates(
            helper_generics(input),
            index.index.fields.iter().map(|field| {
                let ty = &field.ty;
                parse_quote!(#ty: 'static)
            }),
        );
        let (spec_impl_generics, _, spec_where_clause) = spec_generics.split_for_impl();
        quote! {
            #vis type #kind = <#selector as ::multi_index_map::MultiIndexSelector>::Kind;
            #vis struct #spec;

            impl #spec_impl_generics ::multi_index_map::__private::IndexSpec<#node_ty>
                for #spec #spec_where_clause
            {
                type Key<#key_lifetime> = #borrowed_key;
                type Link = <#kind as ::multi_index_map::__private::IndexCategory>::Link;
                const NAME: &'static str =
                    <#selector as ::multi_index_map::MultiIndexSelector>::NAME;

                fn key<#key_lifetime>(
                    value: &#key_lifetime #element,
                ) -> Self::Key<#key_lifetime> {
                    #key_expr
                }

                fn link<#key_lifetime>(node: &#key_lifetime #node_ty) -> &#key_lifetime Self::Link {
                    &node.#link
                }

                fn link_mut<#key_lifetime>(
                    node: &#key_lifetime mut #node_ty,
                ) -> &#key_lifetime mut Self::Link {
                    &mut node.#link
                }
            }
        }
    });

    quote! {
        #vis struct #node #node_generics #where_clause {
            value: #element,
            #(#links,)*
        }

        impl #impl_generics #node #type_args #where_clause {
            fn new(value: #element) -> Self {
                Self {
                    value,
                    #(#defaults,)*
                }
            }
        }

        impl #impl_generics ::multi_index_map::__private::NodeValue for #node_ty #where_clause {
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
    let element = applied(&names.element, input);
    let lifetime = &names.update_lifetime;
    let generics = with_predicates(
        with_lifetime(helper_generics(input), lifetime),
        input.unindexed.iter().map(|field| {
            let ty = &field.ty;
            parse_quote!(#ty: #lifetime)
        }),
    );
    let where_clause = &generics.where_clause;
    let fields = input.unindexed.iter().map(|field| {
        let vis = &field.vis;
        let ident = &field.ident;
        let ty = &field.ty;
        quote!(#vis #ident: &#lifetime mut #ty)
    });
    quote! {
        #[allow(dead_code)]
        #vis struct #update #generics #where_clause {
            #(#fields,)*
            #[doc(hidden)]
            _marker: ::std::marker::PhantomData<(&#lifetime mut (), fn() -> #element)>,
        }
    }
}

fn generate_iterators(input: &Input, names: &Names, indexes: &[IndexNames<'_>]) -> TokenStream {
    let refs = &names.refs;
    let view_lifetime = &names.view_lifetime;
    let iter_param = &names.iter_param;
    let node_param = &names.node_param;
    let wrappers = indexes
        .iter()
        .map(|index| generate_index_iterators(input, names, index));
    quote! {
        struct #refs<#view_lifetime, #node_param, #iter_param> {
            nodes: &#view_lifetime ::multi_index_map::__private::Slab<#node_param>,
            ids: #iter_param,
        }

        impl<#view_lifetime, #node_param, #iter_param> #refs<#view_lifetime, #node_param, #iter_param> {
            fn new(
                nodes: &#view_lifetime ::multi_index_map::__private::Slab<#node_param>,
                ids: #iter_param,
            ) -> Self {
                Self { nodes, ids }
            }
        }

        impl<#view_lifetime, #node_param, #iter_param> Iterator
            for #refs<#view_lifetime, #node_param, #iter_param>
        where
            #node_param: ::multi_index_map::__private::NodeValue,
            #iter_param: Iterator<Item = ::multi_index_map::__private::NodeId>,
        {
            type Item = &#view_lifetime <#node_param as ::multi_index_map::__private::NodeValue>::Value;

            fn next(&mut self) -> Option<Self::Item> {
                self.ids.next().map(|id| self.nodes[id.0].value())
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                self.ids.size_hint()
            }
        }

        impl<#node_param, #iter_param> DoubleEndedIterator for #refs<'_, #node_param, #iter_param>
        where
            #node_param: ::multi_index_map::__private::NodeValue,
            #iter_param: DoubleEndedIterator<Item = ::multi_index_map::__private::NodeId>,
        {
            fn next_back(&mut self) -> Option<Self::Item> {
                self.ids.next_back().map(|id| self.nodes[id.0].value())
            }
        }

        impl<#node_param, #iter_param> ExactSizeIterator for #refs<'_, #node_param, #iter_param>
        where
            #node_param: ::multi_index_map::__private::NodeValue,
            #iter_param: ExactSizeIterator<Item = ::multi_index_map::__private::NodeId>,
        {}

        impl<#node_param, #iter_param> ::std::iter::FusedIterator for #refs<'_, #node_param, #iter_param>
        where
            #node_param: ::multi_index_map::__private::NodeValue,
            #iter_param: ::std::iter::FusedIterator<Item = ::multi_index_map::__private::NodeId>,
        {}

        #(#wrappers)*
    }
}

fn generate_index_iterators(input: &Input, names: &Names, index: &IndexNames<'_>) -> TokenStream {
    let vis = input.child_visibility();
    let element = applied(&names.element, input);
    let refs = &names.refs;
    let node = applied(&names.node, input);
    let iter = &index.iter;
    let equal = &index.equal;
    let range = &index.range;
    let lifetime = &names.view_lifetime;
    let ids = &names.iter_param;
    let base = with_type(with_lifetime(helper_generics(input), lifetime), ids);
    let iterator_generics = with_predicates(
        base.clone(),
        [
            parse_quote!(#node: #lifetime),
            parse_quote!(#ids: Iterator<Item = ::multi_index_map::__private::NodeId>),
        ],
    );
    let double_generics = with_predicates(
        base,
        [
            parse_quote!(#node: #lifetime),
            parse_quote!(
                #ids: DoubleEndedIterator<Item = ::multi_index_map::__private::NodeId>
            ),
        ],
    );
    let (iterator_impl, iterator_ty, iterator_where) = iterator_generics.split_for_impl();
    let (double_impl, double_ty, double_where) = double_generics.split_for_impl();

    quote! {
        #[doc(hidden)]
        #vis struct #iter #iterator_generics #iterator_where {
            inner: #refs<#lifetime, #node, #ids>,
        }

        impl #iterator_impl Iterator for #iter #iterator_ty #iterator_where {
            type Item = &#lifetime #element;
            fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
            fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
        }

        impl #double_impl DoubleEndedIterator for #iter #double_ty #double_where {
            fn next_back(&mut self) -> Option<Self::Item> { self.inner.next_back() }
        }

        #[doc(hidden)]
        #vis struct #equal #iterator_generics #iterator_where {
            inner: #refs<#lifetime, #node, #ids>,
        }

        impl #iterator_impl Iterator for #equal #iterator_ty #iterator_where {
            type Item = &#lifetime #element;
            fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
            fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
        }

        impl #double_impl DoubleEndedIterator for #equal #double_ty #double_where {
            fn next_back(&mut self) -> Option<Self::Item> { self.inner.next_back() }
        }

        #[doc(hidden)]
        #vis struct #range #double_generics #double_where {
            inner: #refs<#lifetime, #node, #ids>,
        }

        impl #double_impl Iterator for #range #double_ty #double_where {
            type Item = &#lifetime #element;
            fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
            fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
        }

        impl #double_impl DoubleEndedIterator for #range #double_ty #double_where {
            fn next_back(&mut self) -> Option<Self::Item> { self.inner.next_back() }
        }
    }
}

fn generate_map(input: &Input, names: &Names, indexes: &[IndexNames<'_>]) -> TokenStream {
    let vis = input.child_visibility();
    let element = applied(&names.element, input);
    let map = &names.map;
    let inner = &names.inner;
    let node = applied(&names.node, input);
    let update = &names.update;
    let selector = &names.selector;
    let map_ty = applied(map, input);
    let inner_ty = applied(inner, input);
    let selector_ty = applied(selector, input);
    let selector_param = &names.selector_param;
    let map_decl_generics = map_generics(input, names, indexes, true);
    let map_helper_generics = map_generics(input, names, indexes, false);
    let map_decl_where = &map_decl_generics.where_clause;
    let map_helper_where = &map_helper_generics.where_clause;
    let (map_impl_generics, _, map_where_clause) = map_helper_generics.split_for_impl();
    let selector_generics = map_helper_generics.clone();
    let selector_where = &selector_generics.where_clause;
    let view_lifetime = &names.view_lifetime;
    let update_ty = application(update, input, [quote!('_)], []);
    let update_static_bounds = input.unindexed.iter().map(|field| {
        let ty = &field.ty;
        quote!(#ty: 'static,)
    });
    let update_static_bounds = update_static_bounds.collect::<Vec<_>>();

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
        let selector = index.selector();
        let kind = &index.kind;
        let view = &index.view;
        let view_mut = &index.view_mut;
        let view_ty = application(view, input, [quote!(#view_lifetime)], [quote!(#kind)]);
        let view_mut_ty = application(view_mut, input, [quote!(#view_lifetime)], [quote!(#kind)]);
        quote! {
            impl #map_impl_generics #selector_ty for #selector #map_where_clause {
                type View<#view_lifetime> = #view_ty
                where
                    #map_ty: #view_lifetime;
                type ViewMut<#view_lifetime> = #view_mut_ty
                where
                    #map_ty: #view_lifetime;

                fn view<#view_lifetime>(
                    map: &#view_lifetime #map_ty,
                ) -> Self::View<#view_lifetime>
                where
                    #map_ty: #view_lifetime,
                {
                    #view { map, marker: ::std::marker::PhantomData }
                }

                fn view_mut<#view_lifetime>(
                    map: &#view_lifetime mut #map_ty,
                ) -> Self::ViewMut<#view_lifetime>
                where
                    #map_ty: #view_lifetime,
                {
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
        let selector = index.selector();
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
                return Some(<#selector as ::multi_index_map::MultiIndexSelector>::NAME);
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
        let selector = index.selector();
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
                conflict = Some(<#selector as ::multi_index_map::MultiIndexSelector>::NAME);
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
    let bindings = indexes.iter().map(|index| {
        let binding = &index.binding;
        let kind_param = &names.kind;
        let concrete_kind = &index.kind;
        let spec = &index.spec;
        let storage = &index.storage;
        let binding_generics = with_predicates(
            with_type(helper_generics(input), kind_param),
            [
                parse_quote!(
                    #kind_param: ::multi_index_map::__private::IndexCategory<
                        Link = <#concrete_kind as ::multi_index_map::__private::IndexCategory>::Link
                    >
                ),
                parse_quote!(
                    #kind_param: ::multi_index_map::__private::IndexKind<#node, #spec>
                ),
            ]
            .into_iter()
            .chain(index.index.fields.iter().map(|field| {
                let ty = &field.ty;
                parse_quote!(#ty: 'static)
            })),
        );
        let binding_where = &binding_generics.where_clause;
        let concrete_binding = application(binding, input, [], [quote!(#concrete_kind)]);
        quote! {
            trait #binding #binding_generics #binding_where {
                fn index(&self) -> &<#kind_param as
                    ::multi_index_map::__private::IndexKind<#node, #spec>>::Index;
            }

            impl #map_impl_generics #concrete_binding for #inner_ty #map_where_clause {
                fn index(&self) -> &<#concrete_kind as
                    ::multi_index_map::__private::IndexKind<#node, #spec>>::Index {
                    &self.#storage
                }
            }
        }
    });

    quote! {
        #(#bindings)*

        #vis trait #selector #selector_generics: ::multi_index_map::MultiIndexSelector #selector_where {
            type View<#view_lifetime>
            where
                Self: #view_lifetime,
                #map_ty: #view_lifetime;
            type ViewMut<#view_lifetime>
            where
                Self: #view_lifetime,
                #map_ty: #view_lifetime;
            fn view<#view_lifetime>(
                map: &#view_lifetime #map_ty,
            ) -> Self::View<#view_lifetime>
            where
                #map_ty: #view_lifetime;
            fn view_mut<#view_lifetime>(
                map: &#view_lifetime mut #map_ty,
            ) -> Self::ViewMut<#view_lifetime>
            where
                #map_ty: #view_lifetime;
        }

        #vis struct #map #map_decl_generics #map_decl_where {
            inner: #inner_ty,
        }

        struct #inner #map_helper_generics #map_helper_where {
            nodes: ::multi_index_map::__private::Slab<#node>,
            #(#storages,)*
        }

        impl #map_impl_generics Default for #inner_ty #map_where_clause {
            fn default() -> Self {
                Self {
                    nodes: ::std::default::Default::default(),
                    #(#defaults,)*
                }
            }
        }

        impl #map_impl_generics Default for #map_ty #map_where_clause {
            fn default() -> Self {
                Self {
                    inner: ::std::default::Default::default(),
                }
            }
        }

        impl #map_impl_generics #map_ty #map_where_clause {
            #vis fn new() -> Self { Self::default() }
            #vis fn len(&self) -> usize { self.inner.nodes.len() }
            #vis fn is_empty(&self) -> bool { self.inner.nodes.is_empty() }

            #vis fn by<#selector_param: #selector_ty>(&self) -> #selector_param::View<'_> {
                #selector_param::view(self)
            }

            #vis fn by_mut<#selector_param: #selector_ty>(&mut self) -> #selector_param::ViewMut<'_> {
                #selector_param::view_mut(self)
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

        impl #map_impl_generics #inner_ty #map_where_clause {
            fn try_insert(
                &mut self,
                value: #element,
            ) -> Result<&#element, ::multi_index_map::Conflict<#element>> {
                self.reserve_all();
                let id = ::multi_index_map::__private::NodeId(self.nodes.insert(<#node>::new(value)));
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
                    ::multi_index_map::__private::NodeId(self.nodes.insert(<#node>::new(replacement)));
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
                f: impl FnOnce(#update_ty),
            ) -> &#element
            where
                #(#update_static_bounds)*
            {
                let value = &mut self.nodes[id.0].value;
                f(#update_expr);
                &self.nodes[id.0].value
            }

            fn update_ids(
                &mut self,
                ids: Vec<::multi_index_map::__private::NodeId>,
                mut f: impl FnMut(#update_ty),
            ) -> usize
            where
                #(#update_static_bounds)*
            {
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
    let fields = input.unindexed.iter().map(|field| {
        let ident = &field.ident;
        quote!(#ident: &mut value.#ident)
    });
    quote!(#update {
        #(#fields,)*
        _marker: ::std::marker::PhantomData,
    })
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
    let element = applied(&names.element, input);
    let lifetime = &names.view_lifetime;
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
                assert!(target.is_none(), "compatibility selector targeted a missing node");
                fields
            }
        }
    };
    quote! {
        #update_fields

        fn order_refs_for_ids<#lifetime>(
            &#lifetime self,
            ids: &[::multi_index_map::__private::NodeId],
        ) -> Vec<&#lifetime #element> {
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

fn query(index: &Index, names: &Names) -> Query {
    if let Some(field) = index.single_field() {
        let ty = &field.ty;
        let query = &names.query;
        Query {
            generics: quote!(#query: ?Sized),
            argument: quote!(key: &#query),
            query_type: quote!(#query),
            key_ref: quote!(key),
            ordered_bounds: quote!(
                #ty: ::std::borrow::Borrow<#query>,
                #query: Ord,
            ),
        }
    } else {
        let q = names.query_components[..index.fields.len()]
            .iter()
            .collect::<Vec<_>>();
        let query_lifetime = &names.query_lifetime;
        let types = index
            .fields
            .iter()
            .map(|field| &field.ty)
            .collect::<Vec<_>>();
        Query {
            generics: quote!(#query_lifetime, #(#q: ?Sized + #query_lifetime),*),
            argument: quote!(key: (#(&#query_lifetime #q,)*)),
            query_type: quote!((#(&#query_lifetime #q,)*)),
            key_ref: quote!(&key),
            ordered_bounds: quote!(
                #(#types: ::std::borrow::Borrow<#q>, #q: Ord,)*
            ),
        }
    }
}

fn generate_view(input: &Input, names: &Names, index: &IndexNames<'_>) -> TokenStream {
    let vis = input.child_visibility();
    let element = applied(&names.element, input);
    let map = &names.map;
    let map_ty = applied(map, input);
    let inner = applied(&names.inner, input);
    let node = applied(&names.node, input);
    let update = &names.update;
    let update_ty = application(update, input, [quote!('_)], []);
    let update_static_bounds = input
        .unindexed
        .iter()
        .map(|field| {
            let ty = &field.ty;
            quote!(#ty: 'static,)
        })
        .collect::<Vec<_>>();
    let refs = &names.refs;
    let spec = &index.spec;
    let view = &index.view;
    let view_mut = &index.view_mut;
    let iter = &index.iter;
    let equal = &index.equal;
    let range = &index.range;
    let kind = &names.kind;
    let concrete_kind = &index.kind;
    let binding = application(&index.binding, input, [], [quote!(#kind)]);
    let index_ref = quote!(<#inner as #binding>::index(&self.map.inner));
    let lifetime = &names.view_lifetime;
    let key_lifetime = &names.key_lifetime;
    let range_param = &names.range_param;
    let owned_key = index.owned_key();
    let query = query(index.index, names);
    let q_generics = &query.generics;
    let argument = &query.argument;
    let q_ty = &query.query_type;
    let key_ref = &query.key_ref;
    let ordered_bounds = &query.ordered_bounds;
    let capabilities = generate_capability_traits(input, names, index);

    let all_index_names = input
        .indexes
        .iter()
        .map(|item| IndexNames::new(names, item))
        .collect::<Vec<_>>();
    let map_bounds = engine_predicates(input, names, &all_index_names);
    let view_struct_generics = with_predicates(
        with_type(with_lifetime(helper_generics(input), lifetime), kind),
        map_bounds
            .clone()
            .into_iter()
            .chain([parse_quote!(#map_ty: #lifetime)]),
    );
    let view_struct_where = &view_struct_generics.where_clause;
    let selected_lifetime_generics = with_predicates(
        view_struct_generics.clone(),
        selected_index_predicates(kind, concrete_kind, &node, spec, &inner, &binding),
    );
    let selected_generics = with_predicates(
        with_type(map_generics(input, names, &all_index_names, false), kind),
        selected_index_predicates(kind, concrete_kind, &node, spec, &inner, &binding),
    );
    let unique_lifetime_generics = with_predicates(
        selected_lifetime_generics.clone(),
        [parse_quote!(#kind: ::multi_index_map::__private::UniqueCategory)],
    );
    let unique_generics = with_predicates(
        selected_generics.clone(),
        [parse_quote!(#kind: ::multi_index_map::__private::UniqueCategory)],
    );
    let non_unique_lifetime_generics = with_predicates(
        selected_lifetime_generics.clone(),
        [parse_quote!(#kind: ::multi_index_map::__private::NonUniqueCategory)],
    );
    let non_unique_generics = with_predicates(
        selected_generics.clone(),
        [parse_quote!(#kind: ::multi_index_map::__private::NonUniqueCategory)],
    );
    let ordered_lifetime_generics = with_predicates(
        selected_lifetime_generics.clone(),
        [
            parse_quote!(#kind: ::multi_index_map::__private::OrderedCategory),
            parse_quote!(#kind: ::multi_index_map::__private::OrderedIndexKind<#node, #spec>),
        ],
    );
    let ordered_generics = with_predicates(
        selected_generics.clone(),
        [
            parse_quote!(#kind: ::multi_index_map::__private::OrderedCategory),
            parse_quote!(#kind: ::multi_index_map::__private::OrderedIndexKind<#node, #spec>),
        ],
    );

    let (selected_lifetime_impl, selected_lifetime_ty, selected_lifetime_where) =
        selected_lifetime_generics.split_for_impl();
    let (selected_impl, _, selected_where) = selected_generics.split_for_impl();
    let (unique_lifetime_impl, unique_lifetime_ty, unique_lifetime_where) =
        unique_lifetime_generics.split_for_impl();
    let (unique_impl, _, unique_where) = unique_generics.split_for_impl();
    let (non_unique_lifetime_impl, non_unique_lifetime_ty, non_unique_lifetime_where) =
        non_unique_lifetime_generics.split_for_impl();
    let (non_unique_impl, _, non_unique_where) = non_unique_generics.split_for_impl();
    let (ordered_lifetime_impl, ordered_lifetime_ty, ordered_lifetime_where) =
        ordered_lifetime_generics.split_for_impl();
    let (ordered_impl, _, ordered_where) = ordered_generics.split_for_impl();

    let view_elided = application(view, input, [quote!('_)], [quote!(#kind)]);
    let view_mut_elided = application(view_mut, input, [quote!('_)], [quote!(#kind)]);
    let iter_lifetime = application(
        iter,
        input,
        [quote!(#lifetime)],
        [quote!(
            <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::Ids<#lifetime>
        )],
    );
    let iter_elided = application(
        iter,
        input,
        [quote!('_)],
        [quote!(
            <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::Ids<'_>
        )],
    );
    let equal_lifetime = application(
        equal,
        input,
        [quote!(#lifetime)],
        [quote!(
            <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::EqualIds<#lifetime>
        )],
    );
    let equal_elided = application(
        equal,
        input,
        [quote!('_)],
        [quote!(
            <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::EqualIds<'_>
        )],
    );
    let range_lifetime = application(
        range,
        input,
        [quote!(#lifetime)],
        [quote!(
            <#kind as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::RangeIds<#lifetime>
        )],
    );
    let range_elided = application(
        range,
        input,
        [quote!('_)],
        [quote!(
            <#kind as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::RangeIds<'_>
        )],
    );

    quote! {
        #[doc(hidden)]
        #vis struct #view #view_struct_generics #view_struct_where {
            map: &#lifetime #map_ty,
            marker: ::std::marker::PhantomData<#kind>,
        }

        #[doc(hidden)]
        #vis struct #view_mut #view_struct_generics #view_struct_where {
            map: &#lifetime mut #map_ty,
            marker: ::std::marker::PhantomData<#kind>,
        }

        impl #selected_lifetime_impl #view #selected_lifetime_ty #selected_lifetime_where {
            #vis fn iter(&self) -> #iter_lifetime {
                #iter {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::iter_ids(
                            #index_ref,
                            &self.map.inner.nodes,
                        ),
                    ),
                }
            }
        }

        impl #selected_impl #view_mut_elided #selected_where {
            #vis fn iter(&self) -> #iter_elided {
                #iter {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::iter_ids(
                            #index_ref,
                            &self.map.inner.nodes,
                        ),
                    ),
                }
            }
        }

        impl #unique_lifetime_impl #view #unique_lifetime_ty #unique_lifetime_where {
            #vis fn get<#q_generics>(&self, #argument) -> Option<&#lifetime #element>
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    #index_ref, #key_ref, &self.map.inner.nodes
                ).map(|id| &self.map.inner.nodes[id.0].value)
            }

            #vis fn contains_key<#q_generics>(&self, #argument) -> bool
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    #index_ref, #key_ref, &self.map.inner.nodes
                ).is_some()
            }
        }

        impl #unique_impl #view_mut_elided #unique_where {
            #vis fn get<#q_generics>(&self, #argument) -> Option<&#element>
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    #index_ref, #key_ref, &self.map.inner.nodes
                ).map(|id| &self.map.inner.nodes[id.0].value)
            }

            #vis fn contains_key<#q_generics>(&self, #argument) -> bool
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    #index_ref, #key_ref, &self.map.inner.nodes
                ).is_some()
            }

            #vis fn remove<#q_generics>(&mut self, #argument) -> Option<#element>
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let id = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    #index_ref, #key_ref, &self.map.inner.nodes
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
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let Some(id) = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    #index_ref, #key_ref, &self.map.inner.nodes
                ) else { return Ok(None); };
                self.map.inner.replace_id(id, replacement).map(Some)
            }

            #vis fn modify<#q_generics>(
                &mut self,
                #argument,
                f: impl FnOnce(&mut #element),
            ) -> Result<Option<&#element>, ::multi_index_map::Conflict<#element>>
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let Some(id) = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    #index_ref, #key_ref, &self.map.inner.nodes
                ) else { return Ok(None); };
                self.map.inner.modify_id(id, f).map(Some)
            }

            #vis fn update<#q_generics>(
                &mut self,
                #argument,
                f: impl FnOnce(#update_ty),
            ) -> Option<&#element>
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
                #(#update_static_bounds)*
            {
                let id = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::find(
                    #index_ref, #key_ref, &self.map.inner.nodes
                )?;
                Some(self.map.inner.update_id(id, f))
            }
        }

        impl #non_unique_lifetime_impl #view #non_unique_lifetime_ty #non_unique_lifetime_where {
            #vis fn equal_range<#q_generics>(&self, #argument) -> #equal_lifetime
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                #equal {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::equal_iter_ids(
                            #index_ref, #key_ref, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl #non_unique_impl #view_mut_elided #non_unique_where {
            #vis fn equal_range<#q_generics>(&self, #argument) -> #equal_elided
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                #equal {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::equal_iter_ids(
                            #index_ref, #key_ref, &self.map.inner.nodes
                        ),
                    ),
                }
            }

            #vis fn remove_all<#q_generics>(&mut self, #argument) -> Vec<#element>
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let ids = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::equal_ids(
                    #index_ref, #key_ref, &self.map.inner.nodes
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
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
            {
                let ids = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::equal_ids(
                    #index_ref, #key_ref, &self.map.inner.nodes
                );
                self.map.inner.modify_ids(ids, f)
            }

            #vis fn update_all<#q_generics>(
                &mut self,
                #argument,
                f: impl FnMut(#update_ty),
            ) -> usize
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>,
                #(#update_static_bounds)*
            {
                let ids = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #q_ty>>::equal_ids(
                    #index_ref, #key_ref, &self.map.inner.nodes
                );
                self.map.inner.update_ids(ids, f)
            }
        }

        impl #ordered_lifetime_impl #view #ordered_lifetime_ty #ordered_lifetime_where {
            #vis fn range<#q_generics, #range_param>(&self, range_value: #range_param) -> #range_lifetime
            where
                #range_param: ::std::ops::RangeBounds<#q_ty>,
                #ordered_bounds
                for<#key_lifetime> <#spec as ::multi_index_map::__private::IndexSpec<#node>>::Key<#key_lifetime>:
                    ::multi_index_map::__private::Compare<#q_ty>,
            {
                #range {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <#kind as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::range_iter_ids(
                            #index_ref, range_value, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl #ordered_impl #view_mut_elided #ordered_where {
            #vis fn range<#q_generics, #range_param>(&self, range_value: #range_param) -> #range_elided
            where
                #range_param: ::std::ops::RangeBounds<#q_ty>,
                #ordered_bounds
                for<#key_lifetime> <#spec as ::multi_index_map::__private::IndexSpec<#node>>::Key<#key_lifetime>:
                    ::multi_index_map::__private::Compare<#q_ty>,
            {
                #range {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <#kind as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::range_iter_ids(
                            #index_ref, range_value, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl #selected_impl ::multi_index_map::IndexView for #view_elided #selected_where {
            type Value = #element;
            type Key = #owned_key;
            type Iter<#lifetime> = #iter_lifetime
            where
                Self: #lifetime,
                #element: #lifetime;

            fn len(&self) -> usize {
                <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::len(#index_ref)
            }

            fn iter(&self) -> Self::Iter<'_> { #view::iter(self) }
        }

        impl #selected_impl ::multi_index_map::IndexView for #view_mut_elided #selected_where {
            type Value = #element;
            type Key = #owned_key;
            type Iter<#lifetime> = #iter_lifetime
            where
                Self: #lifetime,
                #element: #lifetime;

            fn len(&self) -> usize {
                <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::len(#index_ref)
            }

            fn iter(&self) -> Self::Iter<'_> { #view_mut::iter(self) }
        }

        #capabilities
    }
}

fn generate_capability_traits(input: &Input, names: &Names, index: &IndexNames<'_>) -> TokenStream {
    let element = applied(&names.element, input);
    let node = applied(&names.node, input);
    let inner = applied(&names.inner, input);
    let update = &names.update;
    let refs = &names.refs;
    let spec = &index.spec;
    let view = &index.view;
    let view_mut = &index.view_mut;
    let equal = &index.equal;
    let range = &index.range;
    let kind = &names.kind;
    let concrete_kind = &index.kind;
    let binding = application(&index.binding, input, [], [quote!(#kind)]);
    let index_ref = quote!(<#inner as #binding>::index(&self.map.inner));
    let lifetime = &names.view_lifetime;
    let update_lifetime = &names.update_lifetime;
    let query_lifetime = &names.query_lifetime;
    let range_param = &names.range_param;
    let (query_ty, query_bound, query_setup, query_ref, ordered_key_bounds, range_value) =
        if let Some(field) = index.index.single_field() {
            let ty = &field.ty;
            (
                quote!(#ty),
                parse_quote!(#kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #ty>),
                TokenStream::new(),
                quote!(key),
                vec![
                    parse_quote!(#ty: Ord),
                    parse_quote!(
                        for<#lifetime> <#spec as ::multi_index_map::__private::IndexSpec<#node>>::Key<#lifetime>:
                            ::multi_index_map::__private::Compare<#ty>
                    ),
                ],
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
            let query_bound: WherePredicate = parse_quote!(
                for<#query_lifetime> #kind: ::multi_index_map::__private::QueryIndexKind<
                    #node,
                    #spec,
                    (#(&#query_lifetime #types,)*)
                >
            );
            let mut ordered_key_bounds = types
                .iter()
                .map(|ty| parse_quote!(#ty: Ord))
                .collect::<Vec<WherePredicate>>();
            ordered_key_bounds.push(parse_quote!(
                for<#lifetime, #query_lifetime>
                    <#spec as ::multi_index_map::__private::IndexSpec<#node>>::Key<#lifetime>:
                        ::multi_index_map::__private::Compare<(#(&#query_lifetime #types,)*)>
            ));
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

    let all_index_names = input
        .indexes
        .iter()
        .map(|item| IndexNames::new(names, item))
        .collect::<Vec<_>>();
    let selected_generics = with_predicates(
        with_type(map_generics(input, names, &all_index_names, false), kind),
        selected_index_predicates(kind, concrete_kind, &node, spec, &inner, &binding),
    );
    let unique_generics = with_predicates(
        selected_generics.clone(),
        [
            parse_quote!(#kind: ::multi_index_map::__private::UniqueCategory),
            query_bound.clone(),
        ],
    );
    let non_unique_generics = with_predicates(
        selected_generics.clone(),
        [
            parse_quote!(#kind: ::multi_index_map::__private::NonUniqueCategory),
            query_bound,
        ],
    );
    let ordered_generics = with_predicates(
        selected_generics,
        [
            parse_quote!(#kind: ::multi_index_map::__private::OrderedCategory),
            parse_quote!(#kind: ::multi_index_map::__private::OrderedIndexKind<#node, #spec>),
        ]
        .into_iter()
        .chain(ordered_key_bounds),
    );
    let update_static_predicates = input.unindexed.iter().map(|field| {
        let ty = &field.ty;
        parse_quote!(#ty: 'static)
    });
    let update_static_predicates = update_static_predicates.collect::<Vec<WherePredicate>>();
    let unique_mut_generics =
        with_predicates(unique_generics.clone(), update_static_predicates.clone());
    let non_unique_mut_generics =
        with_predicates(non_unique_generics.clone(), update_static_predicates);
    let (unique_impl, _, unique_where) = unique_generics.split_for_impl();
    let (non_unique_impl, _, non_unique_where) = non_unique_generics.split_for_impl();
    let (unique_mut_impl, _, unique_mut_where) = unique_mut_generics.split_for_impl();
    let (non_unique_mut_impl, _, non_unique_mut_where) = non_unique_mut_generics.split_for_impl();
    let (ordered_impl, _, ordered_where) = ordered_generics.split_for_impl();
    let view_elided = application(view, input, [quote!('_)], [quote!(#kind)]);
    let view_mut_elided = application(view_mut, input, [quote!('_)], [quote!(#kind)]);
    let update_gat = application(update, input, [quote!(#update_lifetime)], []);
    let equal_gat = application(
        equal,
        input,
        [quote!(#lifetime)],
        [quote!(
            <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::EqualIds<#lifetime>
        )],
    );
    let range_gat = application(
        range,
        input,
        [quote!(#lifetime)],
        [quote!(
            <#kind as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::RangeIds<#lifetime>
        )],
    );

    quote! {
        impl #unique_impl ::multi_index_map::UniqueView for #view_elided #unique_where {
            fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
                #query_setup
                <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    #index_ref, #query_ref, &self.map.inner.nodes
                ).map(|id| &self.map.inner.nodes[id.0].value)
            }
        }

        impl #unique_impl ::multi_index_map::UniqueView for #view_mut_elided #unique_where {
            fn get(&self, key: &Self::Key) -> Option<&Self::Value> {
                #query_setup
                <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    #index_ref, #query_ref, &self.map.inner.nodes
                ).map(|id| &self.map.inner.nodes[id.0].value)
            }
        }

        impl #unique_mut_impl ::multi_index_map::UniqueViewMut for #view_mut_elided #unique_mut_where {
            type Conflict = ::multi_index_map::Conflict<#element>;
            type Update<#update_lifetime> = #update_gat;

            fn remove(&mut self, key: &Self::Key) -> Option<Self::Value> {
                #query_setup
                let id = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    #index_ref, #query_ref, &self.map.inner.nodes
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
                let Some(id) = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    #index_ref, #query_ref, &self.map.inner.nodes
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
                let Some(id) = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    #index_ref, #query_ref, &self.map.inner.nodes
                ) else { return Ok(None); };
                self.map.inner.modify_id(id, f).map(Some)
            }

            fn update<F>(&mut self, key: &Self::Key, f: F) -> Option<&Self::Value>
            where
                F: for<#update_lifetime> FnOnce(Self::Update<#update_lifetime>),
            {
                #query_setup
                let id = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::find(
                    #index_ref, #query_ref, &self.map.inner.nodes
                )?;
                Some(self.map.inner.update_id(id, f))
            }
        }

        impl #non_unique_impl ::multi_index_map::NonUniqueView for #view_elided #non_unique_where {
            type EqualRange<#lifetime> = #equal_gat
            where
                Self: #lifetime,
                #element: #lifetime;

            fn equal_range(&self, key: &Self::Key) -> Self::EqualRange<'_> {
                #query_setup
                #equal {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::equal_iter_ids(
                            #index_ref, #query_ref, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl #non_unique_impl ::multi_index_map::NonUniqueView for #view_mut_elided #non_unique_where {
            type EqualRange<#lifetime> = #equal_gat
            where
                Self: #lifetime,
                #element: #lifetime;

            fn equal_range(&self, key: &Self::Key) -> Self::EqualRange<'_> {
                #query_setup
                #equal {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::equal_iter_ids(
                            #index_ref, #query_ref, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl #non_unique_mut_impl ::multi_index_map::NonUniqueViewMut for #view_mut_elided #non_unique_mut_where {
            type ModifyAllResult = ::multi_index_map::ModifyAllResult<#element>;
            type Update<#update_lifetime> = #update_gat;

            fn remove_all(&mut self, key: &Self::Key) -> Vec<Self::Value> {
                #query_setup
                let ids = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::equal_ids(
                    #index_ref, #query_ref, &self.map.inner.nodes
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
                let ids = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::equal_ids(
                    #index_ref, #query_ref, &self.map.inner.nodes
                );
                self.map.inner.modify_ids(ids, f)
            }

            fn update_all<F>(&mut self, key: &Self::Key, f: F) -> usize
            where
                F: for<#update_lifetime> FnMut(Self::Update<#update_lifetime>),
            {
                #query_setup
                let ids = <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query_ty>>::equal_ids(
                    #index_ref, #query_ref, &self.map.inner.nodes
                );
                self.map.inner.update_ids(ids, f)
            }
        }

        impl #ordered_impl ::multi_index_map::OrderedView for #view_elided #ordered_where {
            type Range<#lifetime> = #range_gat
            where
                Self: #lifetime,
                #element: #lifetime;

            fn range<#range_param>(&self, range_value: #range_param) -> Self::Range<'_>
            where
                #range_param: ::std::ops::RangeBounds<Self::Key>,
            {
                #range {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <#kind as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::range_iter_ids(
                            #index_ref, #range_value, &self.map.inner.nodes
                        ),
                    ),
                }
            }
        }

        impl #ordered_impl ::multi_index_map::OrderedView for #view_mut_elided #ordered_where {
            type Range<#lifetime> = #range_gat
            where
                Self: #lifetime,
                #element: #lifetime;

            fn range<#range_param>(&self, range_value: #range_param) -> Self::Range<'_>
            where
                #range_param: ::std::ops::RangeBounds<Self::Key>,
            {
                #range {
                    inner: #refs::new(
                        &self.map.inner.nodes,
                        <#kind as ::multi_index_map::__private::OrderedIndexKind<#node, #spec>>::range_iter_ids(
                            #index_ref, #range_value, &self.map.inner.nodes
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
    let element = applied(&names.element, input);
    let map = &names.map;
    let map_ty = applied(map, input);
    let inner = applied(&names.inner, input);
    let node = applied(&names.node, input);
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
    let query = &names.query;
    let map_generics = map_generics(
        input,
        names,
        &input
            .indexes
            .iter()
            .map(|item| IndexNames::new(names, item))
            .collect::<Vec<_>>(),
        false,
    );
    let (map_impl, _, map_where) = map_generics.split_for_impl();
    let iter_ty = application(
        iter,
        input,
        [quote!('_)],
        [quote!(
            <#kind as ::multi_index_map::__private::IndexKind<#node, #spec>>::Ids<'_>
        )],
    );
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
    let update_static_bounds = input
        .unindexed
        .iter()
        .map(|field| {
            let ty = &field.ty;
            quote!(#ty: 'static,)
        })
        .collect::<Vec<_>>();

    quote! {
        impl #map_impl #map_ty #map_where {
            #[deprecated(note = "use map.by::<Selector>().get/equal_range(key)")]
            #field_vis fn #get_by<#query: ?Sized>(&self, key: &#query) -> #collection<&#element>
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query>,
            {
                let values =
                    <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query>>::equal_ids(
                        &self.inner.#storage, key, &self.inner.nodes
                    )
                    .into_iter()
                    .map(|id| &self.inner.nodes[id.0].value)
                    .collect();
                <#kind as ::multi_index_map::__private::CompatibilityKind>::from_vec(values)
            }

            #[deprecated(note = "use map.by_mut::<Selector>().update/update_all(key, ...)")]
            #field_vis fn #get_mut_by(&mut self, key: &#ty) -> #collection<#tuple_type> {
                let ids =
                    <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #ty>>::equal_ids(
                        &self.inner.#storage, key, &self.inner.nodes
                    );
                let fields = self.inner.update_fields_for_ids(ids);
                <#kind as ::multi_index_map::__private::CompatibilityKind>::from_vec(fields)
            }

            #[deprecated(note = "use map.by_mut::<Selector>().modify/modify_all(key, ...)")]
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
                <#inner>::panic_on_modify_conflicts(result);
                let values = self.inner.order_refs_for_ids(&ids);
                <#kind as ::multi_index_map::__private::CompatibilityKind>::from_vec(values)
            }

            #[deprecated(note = "use map.by_mut::<Selector>().update/update_all(key, ...)")]
            #field_vis fn #update_by<#query: ?Sized>(
                &mut self,
                key: &#query,
                mut f: impl FnMut(#(#update_types),*),
            ) -> #collection<&#element>
            where
                #kind: ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query>,
                #(#update_static_bounds)*
            {
                let ids =
                    <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #query>>::equal_ids(
                        &self.inner.#storage, key, &self.inner.nodes
                    );
                for id in &ids {
                    self.inner.update_id(*id, |fields| f(#(#update_args),*));
                }
                let values = self.inner.order_refs_for_ids(&ids);
                <#kind as ::multi_index_map::__private::CompatibilityKind>::from_vec(values)
            }

            #[deprecated(note = "use map.by_mut::<Selector>().remove/remove_all(key)")]
            #field_vis fn #remove_by(&mut self, key: &#ty) -> #collection<#element> {
                let ids =
                    <#kind as ::multi_index_map::__private::QueryIndexKind<#node, #spec, #ty>>::equal_ids(
                        &self.inner.#storage, key, &self.inner.nodes
                    );
                let values = ids.into_iter().map(|id| self.inner.remove_id(id)).collect();
                self.inner.validate_debug();
                <#kind as ::multi_index_map::__private::CompatibilityKind>::from_vec(values)
            }

            #[deprecated(note = "use map.by::<Selector>().iter()")]
            #field_vis fn #iter_by(&self) -> #iter_ty {
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
