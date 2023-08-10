use ::proc_macro_error::{abort_call_site, proc_macro_error};
use ::quote::format_ident;
use ::syn::{parse_macro_input, DeriveInput};

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
    // otherwise throw an error as we do not support Unnamed of Unit structs.
    let named_fields = match fields {
        syn::Fields::Named(f) => f,
        _ => abort_call_site!(
            "Struct fields must be named, unnamed tuple structs and unit structs are not supported"
        ),
    };

    // Filter out all the fields that do not have a multi_index attribute,
    // so we can ignore the non-indexed fields.
    let fields_to_index = named_fields
        .named
        .iter()
        .filter(|f| f.attrs.iter().any(|attr| attr.path.is_ident("multi_index")))
        .collect::<Vec<_>>();

    let lookup_table_fields = generators::generate_lookup_tables(&fields_to_index);

    let lookup_table_fields_init = generators::generate_lookup_table_init(&fields_to_index);

    let lookup_table_fields_reserve = generators::generate_lookup_table_reserve(&fields_to_index);

    let lookup_table_fields_shrink = generators::generate_lookup_table_shrink(&fields_to_index);

    let inserts = generators::generate_inserts(&fields_to_index);

    let removes = generators::generate_removes(&fields_to_index);

    let modifies = generators::generate_modifies(&fields_to_index);

    let clears = generators::generate_clears(&fields_to_index);

    let element_name = &input.ident;

    let map_name = format_ident!("MultiIndex{}Map", element_name);

    let accessors = generators::generate_accessors(
        &fields_to_index,
        &map_name,
        element_name,
        &removes,
        &modifies,
    );

    let iterators = generators::generate_iterators(&fields_to_index, &map_name, element_name);

    let element_vis = input.vis;

    let expanded = generators::generate_expanded(
        &map_name,
        element_name,
        &element_vis,
        &inserts,
        &accessors,
        &iterators,
        &clears,
        &lookup_table_fields,
        &lookup_table_fields_init,
        &lookup_table_fields_shrink,
        &lookup_table_fields_reserve,
    );

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}
