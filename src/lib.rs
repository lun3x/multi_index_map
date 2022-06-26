use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(MultiIndexMap)]
pub fn multi_index_map(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Extract the struct fields if we are parsing a struct, otherwise throw an error as we do not support Enums or Unions
    let fields = match input.data {
        syn::Data::Struct(d) => d.fields,
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };

    // For each field generate a TokenStream representing the mapped index to the main store
    let tokens: Vec<quote::__private::TokenStream> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| {
                let index = format_ident!("{}_index", f.ident.as_ref().unwrap());
                let ty = &f.ty;

                quote! {
                    #index: FxHashMap<#ty, u64>
                }
            })
            .collect()
    } else {
        todo!()
    };

    // Generate the name of the MultiIndexMap
    let name = input.ident;
    let map_name = format_ident!("MultiIndex{}Map", name);

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        #[derive(Debug)]
        struct #map_name {
            store: FxHashMap<u64, #name>,
            #(#tokens),*
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
