use proc_macro::TokenStream;
use syn::parse_macro_input;

mod generate;
mod model;
mod selector;

#[proc_macro_derive(MultiIndexSelector, attributes(multi_index))]
pub fn multi_index_selector(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match selector::generate(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

#[proc_macro_derive(MultiIndexMap, attributes(multi_index))]
pub fn multi_index_map(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    match model::Input::parse(input).map(generate::generate) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}
