use ::proc_macro_error::{abort_call_site, proc_macro_error};
use ::quote::format_ident;
use ::syn::{parse_macro_input, DeriveInput};
use proc_macro_error::OptionExt;

mod generators;
mod index_attributes;

#[proc_macro_derive(MultiIndexMap, attributes(multi_index))]
#[proc_macro_error]
pub fn multi_index_map(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

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
            let kind = index_attributes::get_index_kind(&f);
            (f, kind)
        })
        .partition(|(_, kind)| kind.is_some());

    // Massage the two partitioned Vecs into the correct types
    let indexed_fields = indexed_fields
        .into_iter()
        .map(|(field, kind)| {
            let (ordering, uniqueness) = kind
                .expect_or_abort("Internal logic broken, all indexed fields should have a kind");
            (field, ordering, uniqueness)
        })
        .collect::<Vec<_>>();

    let unindexed_fields = unindexed_fields
        .into_iter()
        .map(|(field, _)| field)
        .collect::<Vec<_>>();

    let lookup_table_fields = generators::generate_lookup_tables(&indexed_fields);

    let lookup_table_fields_init = generators::generate_lookup_table_init(&indexed_fields);

    let lookup_table_fields_reserve = generators::generate_lookup_table_reserve(&indexed_fields);

    let lookup_table_fields_shrink = generators::generate_lookup_table_shrink(&indexed_fields);

    let inserts = generators::generate_inserts(&indexed_fields);

    let removes = generators::generate_removes(&indexed_fields);

    let pre_modifies = generators::generate_pre_modifies(&indexed_fields);

    let post_modifies = generators::generate_post_modifies(&indexed_fields);

    let clears = generators::generate_clears(&indexed_fields);

    let element_name = &input.ident;

    let map_name = format_ident!("MultiIndex{}Map", element_name);

    let accessors = generators::generate_accessors(
        &indexed_fields,
        &unindexed_fields,
        &map_name,
        element_name,
        &removes,
        &pre_modifies,
        &post_modifies,
    );

    let iterators = generators::generate_iterators(&indexed_fields, &map_name, element_name);

    let element_vis = input.vis;

    let expanded = generators::generate_expanded(
        &map_name,
        element_name,
        &element_vis,
        inserts,
        accessors,
        iterators,
        clears,
        lookup_table_fields,
        lookup_table_fields_init,
        lookup_table_fields_shrink,
        lookup_table_fields_reserve,
    );

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}
