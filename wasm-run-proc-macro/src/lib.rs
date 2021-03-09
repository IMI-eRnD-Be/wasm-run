//! See the crate `wasm-run` for documentation.

mod attr_parser;
mod main_generator;

use cargo_metadata::MetadataCommand;
use proc_macro::TokenStream;
use syn::{parse_macro_input, ItemEnum};

/// Makes an entrypoint to your binary (not WASM).
///
/// ```ignore
/// #[wasm_run::main(
///     pre_build = my_pre_build_function,
///     post_build = my_post_build_function,
///     build_args = my_build_args_struct,
///     serve_args = my_serve_args_struct,
///     // ... see below for all the available arguments
/// )]
/// enum Cli {}
/// ```
///
/// It requires to be used with `structopt` on an `enum`. Please consult the documentation of
/// `structopt` if you don't know how to make an `enum` with it.
///
/// There are a number of named arguments you can provide to the macro:
///  -  `other_cli_commands`: a function that is called if you have added new commands to the
///     `enum`;
///  -  `pre_build`: a function that is called when the build has not yet started (you can tweak
///     the command-line arguments of the build command);
///  -  `post_build`: a function that is called when the build is finished (after the optimization
///     with `wasm-opt`);
///  -  `frontend_watch`: a function that is called when the watcher is being initialized (allowing
///      you to add extra things to watch for example);
///  -  `backend_watch`: a function that is called when the watcher is being initialized (allowing
///      you to add extra things to watch for example);
///  -  `serve`: (only if built with the `serve` feature): a function that is called when the HTTP
///     serve is getting configured;
///  -  `default_build_path`: a function that is called that provides the default directory path
///     when the user didn't provide it through the command-line arguments (the default is
///     `workspace root/build`);
///  -  `build_args`: allow you to override the `build` command when providing a custom argument
///     (the default is `DefaultBuildArgs`);
///  -  `serve_args`: allow you to override the `serve` command when providing a custom argument
///     (the default is `DefaultServeArgs`).
///
/// You can also change the frontend package that is built by providing its name in the first
/// positional argument:
///
/// ```ignore
/// #[wasm_run::main("my-frontend-package")]
/// enum Cli {}
/// ```
///
/// And the backend package in the second positional argument:
///
/// ```ignore
/// #[wasm_run::main("my-frontend-package", "my-backend-package")]
/// enum Cli {}
/// ```
///
/// # Examples
///
/// See the [`examples/`](https://github.com/IMI-eRnD-Be/wasm-run/tree/main/examples/custom-cli-command)
/// directory.
#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as ItemEnum);
    let attr = parse_macro_input!(attr with attr_parser::Attr::parse);
    let metadata = MetadataCommand::new()
        .exec()
        .expect("could not get metadata");

    main_generator::generate(item, attr, &metadata)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}
