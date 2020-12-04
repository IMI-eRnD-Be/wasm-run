//! See the crate `wasm-run` for documentation.

mod attr_parser;
mod main_generator;

use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemEnum};

#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemEnum);
    let attr_parser::Attr {
        other_cli_commands,
        prepare_build,
        post_build,
        serve,
        watch,
    } = parse_macro_input!(attr with attr_parser::Attr::parse);

    main_generator::generate(
        item,
        other_cli_commands,
        prepare_build,
        post_build,
        serve,
        watch,
    )
    .unwrap()
    .into()
}
