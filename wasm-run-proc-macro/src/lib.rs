//! See the crate `wasm-run` for documentation.

mod attr_parser;
mod main_generator;

use cargo_metadata::MetadataCommand;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemEnum};

/// Makes an entrypoint to your binary (not WASM).
///
/// It requires to be used with `structopt` on an `enum`. Please consult the documentation of
/// `structopt` if you don't know how to make an `enum` with it.
///
/// By default it provides a command `Build` and a command `Serve` which you can override simply by
/// providing them manually. Otherwise it uses the defaults (`DefaultBuildArgs` and
/// `DefaultServeArgs`).
///
/// There are a number of argument you can provide to the macro:
///  -  `other_cli_commands`: a function that is called if you have added new commands to the
///     `enum`;
///  -  `post_build`: a function that is called when the build is finished (after the optimization
///     with `wasm-opt`);
///  -  `watch`: a function that is called when the watcher is being initialized (allowing you to
///     add extra things to watch for example);
///  -  `serve`: (only if built with the `serve` feature): a function that is called when the HTTP
///     serve is getting configured.
///  -  `run_server`: (only if built *without* the `serve` feature): a function that is called to
///     run the HTTP server.
///
/// Please check the ["complex"](https://github.com/IMI-eRnD-Be/wasm-run/blob/main/examples/complex.rs)
/// example to see how they can be used.
#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemEnum);
    let attr = parse_macro_input!(attr with attr_parser::Attr::parse);
    let metadata = MetadataCommand::new()
        .exec()
        .expect("could not get metadata");

    main_generator::generate(item, attr, &metadata)
        .unwrap()
        .into()
}
