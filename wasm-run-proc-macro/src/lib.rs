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
/// There are a number of named arguments you can provide to the macro:
///  -  `other_cli_commands`: a function that is called if you have added new commands to the
///     `enum`;
///  -  `pre_build`: a function that is called when the build has not yet started (you can tweak
///     the command-line arguments of the build command);
///  -  `post_build`: a function that is called when the build is finished (after the optimization
///     with `wasm-opt`);
///  -  `watch`: a function that is called when the watcher is being initialized (allowing you to
///     add extra things to watch for example);
///  -  `serve`: (only if built with the `serve` feature): a function that is called when the HTTP
///     serve is getting configured;
///  -  `run_server`: (only if built *without* the `serve` feature): a function that is called to
///     run the HTTP server;
///  -  `default_build_path`: a function that is called that provides the default directory path
///     when the user didn't provide it through the command-line arguments (the default is
///     `workspace root/build`).
///  -  `build_args`: Allow you to override the `build` command when providing a custom argument,
///     it replaces `DefaultBuildArgs`.
///  -  `serve_args`: Allow you to override the `serve` command when providing a custom argument,
///     it replaces `DefaultServeArgs`.
/// You can also change the package that is built by providing its name in the first positional
/// argument:
///
/// ```ignore
/// #[wasm_run::main("my-frontend-crate")]
/// ```
///
/// # Example
///
/// Please note that all the code showed here is mostly the actual code for the default hooks.
/// This means that if you don't provide your own hook, this code will be executed. But if you do
/// provide a hook, the code will be *replaced*.
///
/// ```
/// use anyhow::{Context, Result};      // anyhow is provided by `wasm_run::prelude::*`
/// use fs_extra::dir;                  // fs_extra is provided by `wasm_run::prelude::*`
/// use std::fs;
/// use std::path::PathBuf;
/// use structopt::StructOpt;           // due to limitation, this does *not* come from the prelude
/// use wasm_run::prelude::*;
///
/// const DEFAULT_INDEX: &str = r#"<!DOCTYPE html><html><head><meta charset="utf-8"/><script type="module">import init from "/app.js";init();</script></head><body></body></html>"#;
///
/// /// Makes an entrypoint to your binary.
/// #[wasm_run::main(
///     "basic",
///     build_args = BuildCommand,
///     serve_args = ServeCommand,
///     other_cli_commands = run_other_cli_commands,
///     pre_build = pre_build,
///     post_build = post_build,
///     serve = serve,
///     watch = watch,
///     default_build_path = default_build_path,
/// )]
/// #[derive(StructOpt, Debug)]
/// enum Cli {
///     Hello,
/// }
///
/// /// Define a custom `build` command.
/// #[derive(StructOpt, Debug)]
/// struct BuildCommand {
///     #[structopt(skip)]
///     i: i32,
/// }
///
/// impl BuildArgs for BuildCommand {
///     fn build_path(&self) -> &PathBuf {
///         self.default_build_path()
///     }
///
///     fn profiling(&self) -> bool {
///         false
///     }
/// }
///
/// /// Define a custom `serve` command.
/// #[derive(StructOpt, Debug)]
/// struct ServeCommand {
///     #[structopt(flatten)]
///     build_args: BuildCommand,
///
///     #[structopt(skip)]
///     j: i32,
/// }
///
/// impl ServeArgs for ServeCommand {
///     fn build_args(&self) -> &dyn BuildArgs {
///         &self.build_args
///     }
///
///     fn log(&self) -> bool {
///         false
///     }
///
///     fn ip(&self) -> &str {
///         "127.0.0.1"
///     }
///
///     fn port(&self) -> u16 {
///         3000
///     }
/// }
///
/// /// This function is called if you have added new commands to the enum.
/// fn run_other_cli_commands(cli: Cli, _metadata: &Metadata, _package: &Package) -> Result<()> {
///     match cli {
///         Cli::Hello => println!("Hello World!"),
///     }
///
///     Ok(())
/// }
///
/// /// This function is called after the build.
/// fn pre_build(
///     args: &BuildCommand,
///     profile: BuildProfile,
///     command: &mut std::process::Command,
/// ) -> Result<()> {
///     let _i = args.i;
///
///     // NOTE: this is an example, this hook has no default code.
///     command
///         .arg("--no-default-features")
///         .env("RUSTFLAGS", "-Zmacro-backtrace");
///
///     Ok(())
/// }
///
/// /// This function is called after the build.
/// fn post_build(
///     args: &BuildCommand,
///     _profile: BuildProfile,
///     wasm_js: String,
///     wasm_bin: Vec<u8>,
/// ) -> Result<()> {
///     let build_path = args.build_path();
///     let wasm_js_path = build_path.join("app.js");
///     let wasm_bin_path = build_path.join("app_bg.wasm");
///
///     fs::write(&wasm_js_path, wasm_js).with_context(|| {
///         format!("could not write JS file to `{}`", wasm_js_path.display())
///     })?;
///     fs::write(&wasm_bin_path, wasm_bin).with_context(|| {
///         format!("could not write WASM file to `{}`", wasm_bin_path.display())
///     })?;
///
///     let index_path = build_path.join("index.html");
///     let static_dir = args
///         .package()
///         .manifest_path
///         .parent()
///         .unwrap()
///         .join("static");
///
///     if index_path.exists() {
///         fs::copy("index.html", &index_path).context(format!(
///             "could not copy index.html to `{}`",
///             index_path.display()
///         ))?;
///     } else if static_dir.exists() {
///         dir::copy(
///             &static_dir,
///             &build_path,
///             &dir::CopyOptions {
///                 content_only: true,
///                 ..dir::CopyOptions::new()
///             },
///         )
///         .with_context(|| {
///             format!(
///                 "could not copy content of directory static: `{}` to `{}`",
///                 static_dir.display(),
///                 build_path.display()
///             )
///         })?;
///     } else {
///         fs::write(&index_path, DEFAULT_INDEX).with_context(|| {
///             format!(
///                 "could not write default index.html to `{}`",
///                 index_path.display()
///             )
///         })?;
///     }
///
///     #[cfg(feature = "sass")]
///     {
///         let options = args.sass_options(profile);
///         for style_path in args.sass_lookup_directories() {
///             args.build_sass_from_dir(&style_path, options.clone())?;
///         }
///     }
///
///     Ok(())
/// }
///
/// /// This function is called before serving files.
/// fn serve(args: &ServeCommand, server: &mut Server<()>) -> Result<()> {
///     let _j = args.j;
///
///     use tide::{Body, Response};
///
///     let build_path = args.build_args().build_path();
///     let index_path = build_path.join("index.html");
///
///     server.at("/").serve_dir(args.build_args().build_path())?;
///     server.at("/").get(move |_| {
///         let index_path = index_path.clone();
///         async move { Ok(Response::from(Body::from_file(index_path).await?)) }
///     });
///
///     Ok(())
/// }
///
/// /// This function is called when the watcher is being initialized.
/// fn watch(args: &ServeCommand, watcher: &mut RecommendedWatcher) -> Result<()> {
///     let _j = args.j;
///
///     use notify::{RecursiveMode, Watcher};
///     use std::collections::HashSet;
///     use std::iter::FromIterator;
///
///     let metadata = args.build_args().metadata();
///
///     let _ = watcher.watch("index.html", RecursiveMode::Recursive);
///
///     let members: HashSet<_> = HashSet::from_iter(&metadata.workspace_members);
///
///     for package in metadata.packages.iter().filter(|x| members.contains(&x.id)) {
///         let _ = watcher.watch(&package.manifest_path, RecursiveMode::Recursive);
///         let _ = watcher.watch(
///             package.manifest_path.parent().unwrap().join("src"),
///             RecursiveMode::Recursive,
///         );
///     }
///
///     Ok(())
/// }
///
/// /// Define another build path if not provided by the user in the command-line arguments.
/// fn default_build_path(metadata: &Metadata, _package: &Package) -> PathBuf {
///     metadata.workspace_root.join("build")
/// }
/// ```
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
