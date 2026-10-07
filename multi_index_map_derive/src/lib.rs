use ::quote::{format_ident, quote};
use ::syn::parse_quote;
use convert_case::Casing;
use generators::{generate_iter_mut, FieldIdents, EXPECT_NAMED_FIELDS};
use manyhow::{bail, error_message, manyhow};
use syn::visit_mut::VisitMut;

mod generators;
mod index_attributes;
mod private_module;

#[manyhow]
#[proc_macro_derive(
    MultiIndexMap,
    attributes(multi_index, multi_index_derive, multi_index_hash)
)]
pub fn multi_index_map(input: proc_macro::TokenStream) -> syn::Result<proc_macro::TokenStream> {
    // Parse the input tokens into a syntax tree.
    let input = syn::parse(input)?;

    let mut extra_attrs = index_attributes::get_extra_attributes(&input)?;

    // Extract the struct fields if we are parsing a struct,
    // otherwise throw an error as we do not support Enums or Unions.
    let fields = match input.data {
        syn::Data::Struct(d) => d.fields,
        _ => bail!("MultiIndexMap only supports structs as elements"),
    };

    // Verify the struct fields are named fields,
    // otherwise throw an error as we do not support Unnamed or Unit structs.
    let syn::Fields::Named(named_fields) = fields else {
        bail!(
            "Struct fields must be named, unnamed tuple structs and unit structs are not supported"
        )
    };

    let named_fields_with_kind = named_fields
        .named
        .into_iter()
        .map(|f| {
            let index_kind = index_attributes::get_index_kind(&f)?;
            Ok((f, index_kind))
        })
        .collect::<syn::Result<Vec<_>>>()?;

    // Filter out all the fields that do not have a multi_index attribute,
    // so we can ignore the non-indexed fields.
    let (indexed_fields, unindexed_fields): (Vec<_>, Vec<_>) = named_fields_with_kind
        .into_iter()
        .partition(|(_, index_kind)| index_kind.is_some());

    let element_name = &input.ident;

    let map_name = format_ident!("MultiIndex{}Map", element_name);
    let iter_mut_name = format_ident!("{}IterMut", element_name);
    // Keep the element's spelling to avoid collisions between names whose
    // snake_case forms are identical.
    let module_name = format_ident!("__multi_index_map_{}", element_name);

    // Massage the two partitioned Vecs into the correct types
    let mut indexed_fields = indexed_fields
        .into_iter()
        .map(|(field, kind)| -> syn::Result<_> {
            let (ordering, uniqueness) = kind.ok_or_else(|| {
                error_message!("Internal logic broken, all indexed fields should have a kind")
            })?;

            let field_ident = field.ident.as_ref().ok_or_else(|| {
                error_message!("Internal logic broken, all indexed fields should have a name")
            })?;

            let idents = FieldIdents {
                name: field_ident.clone(),
                index_name: format_ident!("_{field_ident}_index",),
                cloned_name: format_ident!("{field_ident}_orig",),
                iter_name: format_ident!(
                    "{map_name}{}Iter",
                    field_ident
                        .to_string()
                        .to_case(::convert_case::Case::UpperCamel),
                ),
            };

            syn::Result::Ok((field, idents, ordering, uniqueness))
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let mut unindexed_fields = unindexed_fields
        .into_iter()
        .map(|(field, _)| field)
        .collect::<Vec<_>>();

    // Re-export each generated type with its original visibility. The internal
    // declarations and methods retain the same accessible scope one level down.
    let element_vis = &input.vis;
    let iterator_exports = indexed_fields.iter().map(|(field, idents, _, _)| {
        let vis = &field.vis;
        let iter_name = &idents.iter_name;
        quote! {
            #[allow(unused_imports)]
            #vis use self::#module_name::#iter_name;
        }
    });
    let exports = quote! {
        #[allow(unused_imports)]
        #element_vis use self::#module_name::{#map_name, #iter_mut_name};
        #(#iterator_exports)*
    };

    let element_vis = private_module::visibility_in_child(&input.vis);
    let mut generics = input.generics;
    let mut parent_scope = private_module::ParentScope::default();
    parent_scope.visit_generics_mut(&mut generics);
    parent_scope.visit_path_mut(&mut extra_attrs.hasher);
    for (field, _, _, _) in &mut indexed_fields {
        field.vis = private_module::visibility_in_child(&field.vis);
        parent_scope.visit_type_mut(&mut field.ty);
    }
    for field in &mut unindexed_fields {
        parent_scope.visit_type_mut(&mut field.ty);
    }

    let lookup_table_fields = generators::generate_lookup_tables(&indexed_fields, &extra_attrs);

    let lookup_table_fields_init = generators::generate_lookup_table_init(&indexed_fields);

    let lookup_table_fields_default = generators::generate_lookup_table_init(&indexed_fields);

    let lookup_table_fields_reserve = generators::generate_lookup_table_reserve(&indexed_fields);

    let lookup_table_fields_shrink = generators::generate_lookup_table_shrink(&indexed_fields);

    let entries_for_insert = generators::generate_entries_for_insert(&indexed_fields);

    let inserts_for_entries = generators::generate_inserts_for_entries(&indexed_fields);

    let removes = generators::generate_removes(&indexed_fields);

    let pre_modifies = generators::generate_pre_modifies(&indexed_fields);

    let post_modifies = generators::generate_post_modifies(&indexed_fields);

    let clears = generators::generate_clears(&indexed_fields);

    let unindexed_types = unindexed_fields.iter().map(|f| &f.ty).collect::<Vec<_>>();
    let unindexed_idents = unindexed_fields
        .iter()
        .map(|f| {
            f.ident
                .as_ref()
                .ok_or_else(|| error_message!("{EXPECT_NAMED_FIELDS}").into())
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let mut iter_generics = generics.clone();
    iter_generics
        .params
        .push(parse_quote!('__mim_iter_lifetime));
    let accessors = generators::generate_accessors(
        &indexed_fields,
        &unindexed_types,
        &unindexed_idents,
        element_name,
        &removes,
        &pre_modifies,
        &post_modifies,
        &generics,
        &iter_generics,
    );

    let iterators =
        generators::generate_iterators(&indexed_fields, element_name, &generics, &iter_generics);

    let iter_mut = generate_iter_mut(
        &iter_mut_name,
        element_name,
        &element_vis,
        &unindexed_types,
        &unindexed_idents,
        &generics,
        &iter_generics,
    );

    let expanded = generators::generate_expanded(
        &extra_attrs,
        &generics,
        &map_name,
        element_name,
        &element_vis,
        entries_for_insert,
        inserts_for_entries,
        accessors,
        iterators,
        clears,
        lookup_table_fields,
        lookup_table_fields_init,
        lookup_table_fields_default,
        lookup_table_fields_shrink,
        lookup_table_fields_reserve,
        &iter_mut_name,
        iter_mut,
        &iter_generics,
    );

    // Hand the output tokens back to the compiler.
    Ok(proc_macro::TokenStream::from(quote! {
        #[allow(non_snake_case)]
        mod #module_name {
            #[allow(unused_imports)]
            use super::*;

            #expanded
        }

        #exports
    }))
}
