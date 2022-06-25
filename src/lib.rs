use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Ident, Type};

#[proc_macro_derive(MultiIndexMap)]
pub fn multi_index_map(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let fields = match input.data {
        syn::Data::Struct(d) => d.fields,
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };

    let idents: Vec<Ident> = if let syn::Fields::Named(f) = &fields {
        f.named
            .iter()
            .map(|f| format_ident!("{}_index", f.ident.as_ref().unwrap()))
            .collect()
    } else {
        todo!()
    };

    let types: Vec<&Type> = if let syn::Fields::Named(f) = &fields {
        f.named.iter().map(|f| &f.ty).collect()
    } else {
        todo!()
    };

    let map_name = format_ident!("MultiIndex{}Map", name);

    // Build the output, possibly using quasi-quotation
    let mut expanded = quote! {
        struct #map_name {
            store: FxHashMap<u64, #name>,
            #(#idents: FxHashMap<#types, u64>),*
        }
    };

    expanded.extend(quote! {});

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}
