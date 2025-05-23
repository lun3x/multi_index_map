use ::proc_macro_error2::{abort_call_site, proc_macro_error};
use ::quote::format_ident;
use ::syn::{parse_macro_input, DeriveInput};
use convert_case::Casing;
use generators::{generate_iter_mut, FieldIdents, EXPECT_NAMED_FIELDS};
use proc_macro_error2::OptionExt;
use syn::parse_quote;

mod generators;
mod index_attributes;

#[proc_macro_derive(
    MultiIndexMap,
    attributes(multi_index, multi_index_derive, multi_index_hash)
)]
#[proc_macro_error]
pub fn multi_index_map(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    let extra_attrs = index_attributes::get_extra_attributes(&input);

    // Extract the struct fields if we are parsing a struct,
    // otherwise throw an error as we do not support Enums or Unions.
    let fields = match input.data {
        syn::Data::Struct(d) => d.fields,
        _ => abort_call_site!("MultiIndexMap only supports structs as elements"),
    };

    // Verify the struct fields are named fields,
    // otherwise throw an error as we do not support Unnamed or Unit structs.
    let named_fields = match fields {
        syn::Fields::Named(f) => f,
        _ => abort_call_site!(
            "Struct fields must be named, unnamed tuple structs and unit structs are not supported"
        ),
    };

    // Filter out all the fields that do not have a multi_index attribute,
    // so we can ignore the non-indexed fields.
    let (indexed_fields, unindexed_fields): (Vec<_>, Vec<_>) = named_fields
        .named
        .into_iter()
        .map(|f| {
            let index_kind = index_attributes::get_index_kind(&f);
            (f, index_kind)
        })
        .partition(|(_, index_kind)| index_kind.is_some());

    let element_name = &input.ident;

    let map_name = format_ident!("MultiIndex{}Map", element_name);

    // Massage the two partitioned Vecs into the correct types
    let indexed_fields = indexed_fields
        .into_iter()
        .map(|(field, kind)| {
            let (ordering, uniqueness) = kind
                .expect_or_abort("Internal logic broken, all indexed fields should have a kind");

            let field_ident = field.ident.as_ref().expect_or_abort(EXPECT_NAMED_FIELDS);

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

            (field, idents, ordering, uniqueness)
        })
        .collect::<Vec<_>>();

    let unindexed_fields = unindexed_fields
        .into_iter()
        .map(|(field, _)| field)
        .collect::<Vec<_>>();

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
        .map(|f| f.ident.as_ref().expect_or_abort(EXPECT_NAMED_FIELDS))
        .collect::<Vec<_>>();

    let mut iter_generics = input.generics.clone();
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
        &input.generics,
        &iter_generics,
    );

    let iterators = generators::generate_iterators(
        &indexed_fields,
        element_name,
        &input.generics,
        &iter_generics,
    );

    let element_vis = input.vis;

    let iter_mut_name = format_ident!("{}IterMut", element_name);
    let iter_mut = generate_iter_mut(
        &iter_mut_name,
        element_name,
        &element_vis,
        &unindexed_types,
        &unindexed_idents,
        &input.generics,
        &iter_generics,
    );

    let expanded = generators::generate_expanded(
        &extra_attrs,
        &input.generics,
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
    proc_macro::TokenStream::from(expanded)
}
