//! ![Rust](https://github.com/IMI-eRnD-Be/wasmbl/workflows/main/badge.svg)
//! [![Latest Version](https://img.shields.io/crates/v/wasmbl.svg)](https://crates.io/crates/wasmbl)
//! [![Docs.rs](https://docs.rs/wasmbl/badge.svg)](https://docs.rs/wasmbl)
//! [![LOC](https://tokei.rs/b1/github/IMI-eRnD-Be/wasmbl)](https://github.com/IMI-eRnD-Be/wasmbl)
//! [![Dependency Status](https://deps.rs/repo/github/IMI-eRnD-Be/wasmbl/status.svg)](https://deps.rs/repo/github/IMI-eRnD-Be/wasmbl)
//! ![License](https://img.shields.io/crates/l/wasmbl)
//!
//! # Synopsis
//!
//! Build tool that replaces `cargo run` to build WASM projects. Just like webpack, `wasmbl`
//! offers a great deal of customization.
//!
//! To build your WASM project you normally need an external tool like `wasm-bindgen`, `wasm-pack`
//! or `cargo-wasm`. `wasmbl` takes a different approach: it's a library that you install as a
//! dependency to your project. Because of that you don't need any external tool, the
//! tooling is built as part of your dependencies, which makes the CI easier to set up and reduce
//! the hassle for new comers to start working on the project.
//!
//! To build your project for production you can use the command `cargo run -- build`. You can also
//! run a development server that rebuilds automatically when the code changes:
//! `cargo run -- serve`. It doesn't rebuild everything, only the backend if the backend changed or
//! the frontend if the frontend changed.
//!
//! **Please note that there is a space between `--` and `build` and between `--` and `serve`!**
//!
//! One of the main advantage of this library is that it provides greater customization: you can
//! set a few hooks during the build process in order to customize the build directory or use a
//! template to generate your index.html, download some CSS, ... you name it. I personally use it
//! to reduce the amount of files by bundling the CSS and the JS into the `index.html` so I had
//! only two files (`index.html`, `app_bg.wasm`).
//!
//! # Examples
//!
//! There are 3 basic examples to help you get started quickly:
//!
//!  -  a ["basic"](https://github.com/IMI-eRnD-Be/wasmbl/tree/main/examples/basic) example for a
//!     frontend only app that rebuilds the app when a file change is detected;
//!  -  a ["backend-and-frontend"](https://github.com/IMI-eRnD-Be/wasmbl/tree/main/examples/backend-and-frontend)
//!     example using the web framework Rocket (backend) which uses Rocket itself to serve the file
//!     during the development (any file change is also detected and it rebuilds and restart
//!     automatically).
//!  -  a ["custom-cli-command"](https://github.com/IMI-eRnD-Be/wasmbl/tree/main/examples/custom-cli-command)
//!     example that adds a custom CLI command named `build-docker-image` which build the backend,
//!     the frontend and package the whole thing in a container image.
//!
//! # Usage
//!
//! All the details about the hooks can be found on the macro [`main`].
//!
//! # Additional Information
//!
//!  *  You can use this library to build examples in the `examples/` directory of your project.
//!     `cargo run --example your_example -- serve`. But you will need to specify the name of the
//!     WASM crate in your project and it must be present in the workspace. Please check the
//!     ["run-an-example"](https://github.com/IMI-eRnD-Be/wasmbl/blob/main/examples/run-an-example.rs)
//!     example.
//!  *  If you want to use your own backend you will need to disable the `dev-server` feature
//!     by disabling the default features. You can use the `full-restart` feature to force the
//!     backend to also be recompiled when a file changes (otherwise only the frontend is
//!     re-compiled). You will also need to specify `run_server` to the macro arguments to run your
//!     backend.
//!  *  You can add commands to the CLI by adding variants in the `enum`.
//!  *  You can add parameters to the `Build` and `Serve` commands by overriding them. Please check
//!     the documentation on the macro `main`.
//!  *  If you run `cargo run -- serve --profiling`, the WASM will be optimized.
//!
//! # Features
//!
//!  *  `prebuilt-wasm-opt`: if you disable the default features and enable this feature, a binary
//!     of wasm-opt will be downloaded from GitHub and used to optimize the WASM. By default,
//!     wasm-opt is compiled among the dependencies (`binaryen`). This is useful if you run into
//!     troubles for building `binaryen-sys`. (`binaryen` cannot be built on Netlify at the
//!     moment.)
//!  *  `sass`: support for SASS and SCSS. All SASS and SCSS files found in the directories
//!     `styles/`, `assets/`, `sass/` and `css/` will be automatically transpiled to CSS and placed
//!     in the build directory. This can be configured by overriding:
//!     [`BuildArgs::build_sass_from_dir`], [`BuildArgs::sass_lookup_directories`],
//!     [`BuildArgs::sass_options`] or completely overriden in the [`Hooks::post_build`] hook.
//!     `sass-rs` is re-exported in the prelude of `wasmbl` for this purpose.
//!  *  `full-restart`: when this feature is active, the command is entirely restarted when changes
//!     are detected when serving files for development (`cargo run -- serve`). This is useful with
//!     custom `serve` command that uses a custom backend and if you need to detect changes in the
//!     backend code itself.

#![warn(missing_docs)]

/// Merge of web-bundler.
///
/// TODO: This is the simple first iteration, we need to integrate properly.
#[cfg(feature = "sass")]
pub mod bundler;
#[cfg(feature = "prebuilt-wasm-opt")]
mod prebuilt_wasm_opt;

use anyhow::{anyhow, bail, Context, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package};
use downcast_rs::*;
use fs_extra::dir;
use log::{error, info, trace, warn};
use notify::RecommendedWatcher;
use once_cell::sync::OnceCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::BufReader;
use std::iter;
use std::iter::FromIterator;
use std::path::{Path, PathBuf};
#[cfg(feature = "dev-server")]
use std::pin::Pin;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc;
use std::time;
use structopt::StructOpt;
#[cfg(feature = "dev-server")]
use tide::Server;

pub use wasmbl_proc_macro::*;

#[doc(hidden)]
pub use structopt;

const DEFAULT_INDEX: &str = r#"<!DOCTYPE html><html><head><meta charset="utf-8"/><script type="module">import init from "/app.js";init(new URL('app_bg.wasm', import.meta.url));</script></head><body></body></html>"#;

static METADATA: OnceCell<Metadata> = OnceCell::new();
static DEFAULT_BUILD_PATH: OnceCell<PathBuf> = OnceCell::new();
static FRONTEND_PACKAGE: OnceCell<&Package> = OnceCell::new();
static BACKEND_PACKAGE: OnceCell<Option<&Package>> = OnceCell::new();
static HOOKS: OnceCell<Hooks> = OnceCell::new();

#[derive(Debug, PartialEq, Clone, Copy)]
/// A build profile for the WASM.
pub enum BuildProfile {
    /// Development profile (no `--release`, no optimization).
    Dev,
    /// Release profile (`--profile`, `-O2 -Os`).
    Release,
    /// Release profile (`--profile`, `-O2 --debuginfo`).
    Profiling,
}

/// This function is called early before any command starts. This is not part of the public API.
#[doc(hidden)]
pub fn wasmbl_init(
    pkg_name: &str,
    backend_pkg_name: Option<&str>,
    default_build_path: Option<Box<dyn FnOnce(&Metadata, &Package) -> PathBuf>>,
    hooks: Hooks,
) -> Result<(&'static Metadata, &'static Package)> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let metadata = MetadataCommand::new()
        .exec()
        .context("this binary is not meant to be ran outside of its workspace")?;

    METADATA
        .set(metadata)
        .expect("the cell is initially empty; qed");

    let metadata = METADATA.get().unwrap();

    let frontend_package = METADATA
        .get()
        .unwrap()
        .packages
        .iter()
        .find(|x| x.name == pkg_name)
        .expect("the frontend package existence has been checked during compile time; qed");

    FRONTEND_PACKAGE
        .set(frontend_package)
        .expect("the cell is initially empty; qed");

    let frontend_package = FRONTEND_PACKAGE.get().unwrap();

    if let Some(name) = backend_pkg_name {
        let backend_package = METADATA
            .get()
            .unwrap()
            .packages
            .iter()
            .find(|x| x.name == name)
            .expect("the backend package existence has been checked during compile time; qed");

        BACKEND_PACKAGE
            .set(Some(backend_package))
            .expect("the cell is initially empty; qed");
    } else {
        BACKEND_PACKAGE
            .set(None)
            .expect("the cell is initially empty; qed");
    }

    DEFAULT_BUILD_PATH
        .set(if let Some(default_build_path) = default_build_path {
            default_build_path(metadata, frontend_package)
        } else {
            metadata.workspace_root.join("build")
        })
        .expect("the cell is initially empty; qed");

    if HOOKS.set(hooks).is_err() {
        panic!("the cell is initially empty; qed");
    }

    Ok((metadata, frontend_package))
}

/// Build arguments.
#[derive(StructOpt, Debug)]
pub struct DefaultBuildArgs {
    /// Build directory output.
    #[structopt(long)]
    pub build_path: Option<PathBuf>,

    /// Create a profiling build. Enable optimizations and debug info.
    #[structopt(long)]
    pub profiling: bool,
}

/// A trait that allows overriding the `build` command.
pub trait BuildArgs: Downcast {
    /// Build directory output.
    fn build_path(&self) -> &PathBuf;

    /// Default path for the build/public directory.
    fn default_build_path(&self) -> &PathBuf {
        DEFAULT_BUILD_PATH
            .get()
            .expect("default_build_path has been initialized on startup; qed")
    }

    /// Path to the `target` directory.
    fn target_path(&self) -> &PathBuf {
        &self.metadata().target_directory
    }

    /// Metadata of the project.
    fn metadata(&self) -> &Metadata {
        METADATA
            .get()
            .expect("metadata has been initialized on startup; qed")
    }

    /// Package metadata.
    fn frontend_package(&self) -> &Package {
        FRONTEND_PACKAGE
            .get()
            .expect("frontend_package has been initialized on startup; qed")
    }

    /// Backend frontend_package metadata.
    fn backend_package(&self) -> Option<&Package> {
        BACKEND_PACKAGE
            .get()
            .expect("frontend_package has been initialized on startup; qed")
            .to_owned()
    }

    /// Create a profiling build. Enable optimizations and debug info.
    fn profiling(&self) -> bool;

    /// Transpile SASS and SCSS files to CSS in the build directory.
    #[cfg(feature = "sass")]
    fn build_sass_from_dir(
        &self,
        input_dir: &std::path::Path,
        options: sass_rs::Options,
    ) -> Result<()> {
        use walkdir::{DirEntry, WalkDir};

        let build_path = self.build_path();

        fn is_sass(entry: &DirEntry) -> bool {
            matches!(
                entry.path().extension().map(|x| x.to_str()).flatten(),
                Some("sass") | Some("scss")
            )
        }

        fn should_ignore(entry: &DirEntry) -> bool {
            entry
                .file_name()
                .to_str()
                .map(|x| x.starts_with("_"))
                .unwrap_or(false)
        }

        let walker = WalkDir::new(&input_dir).into_iter();
        for entry in walker
            .filter_map(|x| match x {
                Ok(x) => Some(x),
                Err(err) => {
                    warn!(
                        "could not walk into directory `{}`: {}",
                        input_dir.display(),
                        err,
                    );
                    None
                }
            })
            .filter(|x| x.path().is_file() && is_sass(x) && !should_ignore(x))
        {
            let file_path = entry.path();
            let css_path = build_path
                .join(file_path.strip_prefix(&input_dir).unwrap())
                .with_extension("css");

            match sass_rs::compile_file(file_path, options.clone()) {
                Ok(css) => {
                    let _ = fs::create_dir_all(css_path.parent().unwrap());
                    fs::write(&css_path, css).with_context(|| {
                        format!("could not write CSS to file `{}`", css_path.display())
                    })?;
                }
                Err(err) => bail!(
                    "could not convert SASS file `{}` to `{}`: {}",
                    file_path.display(),
                    css_path.display(),
                    err,
                ),
            }
        }

        Ok(())
    }

    /// Returns a list of directories to lookup to transpile SASS and SCSS files to CSS.
    #[cfg(feature = "sass")]
    fn sass_lookup_directories(&self, _profile: BuildProfile) -> Vec<PathBuf> {
        const STYLE_CANDIDATES: &[&str] = &["assets", "styles", "css", "sass"];

        let package_path = self.frontend_package().manifest_path.parent().unwrap();

        STYLE_CANDIDATES
            .iter()
            .map(|x| package_path.join(x))
            .filter(|x| x.exists())
            .collect()
    }

    /// Default profile to transpile SASS and SCSS files to CSS.
    #[cfg(feature = "sass")]
    fn sass_options(&self, profile: BuildProfile) -> sass_rs::Options {
        sass_rs::Options {
            output_style: match profile {
                BuildProfile::Release | BuildProfile::Profiling => sass_rs::OutputStyle::Compressed,
                _ => sass_rs::OutputStyle::Nested,
            },
            ..sass_rs::Options::default()
        }
    }

    /// Run the `build` command.
    fn run(self) -> Result<PathBuf>
    where
        Self: Sized + 'static,
    {
        let hooks = HOOKS.get().expect("wasmbl_init() has not been called");
        build(BuildProfile::Release, &self, hooks)?;
        Ok(self.build_path().to_owned())
    }
}

impl_downcast!(BuildArgs);

impl BuildArgs for DefaultBuildArgs {
    fn build_path(&self) -> &PathBuf {
        self.build_path
            .as_ref()
            .unwrap_or_else(|| self.default_build_path())
    }

    fn profiling(&self) -> bool {
        self.profiling
    }
}

/// Serve arguments.
#[derive(StructOpt, Debug)]
pub struct DefaultServeArgs {
    /// Activate HTTP logs.
    #[structopt(long)]
    pub log: bool,

    /// IP address to bind.
    ///
    /// Use 0.0.0.0 to expose the server to your network.
    #[structopt(long, short = "h", default_value = "127.0.0.1")]
    pub ip: String,

    /// Port number.
    #[structopt(long, short = "p", default_value = "3000")]
    pub port: u16,

    /// Build arguments.
    #[structopt(flatten)]
    pub build_args: DefaultBuildArgs,
}

/// A trait that allows overriding the `serve` command.
pub trait ServeArgs: Downcast + Send {
    /// Activate HTTP logs.
    #[cfg(feature = "dev-server")]
    fn log(&self) -> bool;

    /// IP address to bind.
    ///
    /// Use 0.0.0.0 to expose the server to your network.
    #[cfg(feature = "dev-server")]
    fn ip(&self) -> &str;

    /// Port number.
    #[cfg(feature = "dev-server")]
    fn port(&self) -> u16;

    /// Build arguments.
    fn build_args(&self) -> &dyn BuildArgs;

    /// Run the `serve` command.
    fn run(self) -> Result<()>
    where
        Self: Sync + Sized + 'static,
    {
        let hooks = HOOKS.get().expect("wasmbl_init() has not been called");
        // NOTE: the first step for serving is to call `build` a first time. The build directory
        //       must be present before we start watching files there.
        build(BuildProfile::Dev, self.build_args(), hooks)?;
        #[cfg(feature = "dev-server")]
        {
            async_std::task::block_on(async {
                let t1 = async_std::task::spawn(serve_frontend(&self, hooks)?);
                let t2 = async_std::task::spawn_blocking(move || watch_frontend(&self, hooks));
                futures::try_join!(t1, t2)?;
                Err(anyhow!("server and watcher unexpectedly exited"))
            })
        }
        #[cfg(not(feature = "dev-server"))]
        {
            use std::sync::Arc;
            use std::thread;

            if self.build_args().backend_package().is_none() {
                bail!("missing backend crate name");
            }

            let args = Arc::new(self);
            let t1 = {
                let args = Arc::clone(&args);
                thread::spawn(move || watch_frontend(&*args, hooks))
            };
            let t2 = thread::spawn(move || watch_backend(&*args, hooks));
            let _ = t1.join();
            let _ = t2.join();

            Err(anyhow!("server and watcher unexpectedly exited"))
        }
    }
}

impl_downcast!(ServeArgs);

impl ServeArgs for DefaultServeArgs {
    #[cfg(feature = "dev-server")]
    fn log(&self) -> bool {
        self.log
    }

    #[cfg(feature = "dev-server")]
    fn ip(&self) -> &str {
        &self.ip
    }

    #[cfg(feature = "dev-server")]
    fn port(&self) -> u16 {
        self.port
    }

    fn build_args(&self) -> &dyn BuildArgs {
        &self.build_args
    }
}

/// Hooks.
///
/// Check the code of [`Hooks::default()`] implementation to see what they do by default.
///
/// If you don't provide your own hook, the default code will be executed. But if you do provide a
/// hook, the code will be *replaced*.
pub struct Hooks {
    /// This hook will be run before the WASM is compiled. It does nothing by default.
    /// You can tweak the command-line arguments of the build command here or create additional
    /// files in the build directory.
    pub pre_build:
        Box<dyn Fn(&dyn BuildArgs, BuildProfile, &mut Command) -> Result<()> + Send + Sync>,

    /// This hook will be run after the WASM is compiled and optimized.
    /// By default it copies the static files to the build directory.
    #[allow(clippy::type_complexity)]
    pub post_build:
        Box<dyn Fn(&dyn BuildArgs, BuildProfile, String, Vec<u8>) -> Result<()> + Send + Sync>,

    /// This hook will be run before running the HTTP server.
    /// By default it will add routes to the files in the build directory.
    #[cfg(feature = "dev-server")]
    #[allow(clippy::type_complexity)]
    pub serve: Box<dyn Fn(&dyn ServeArgs, &mut Server<()>) -> Result<()> + Send + Sync>,

    /// This hook will be run before starting to watch for changes in files.
    /// By default it will add all the `src/` directories and `Cargo.toml` files of all the crates
    /// in the workspace plus the `static/` directory if it exists in the frontend crate.
    pub frontend_watch:
        Box<dyn Fn(&dyn ServeArgs, &mut RecommendedWatcher) -> Result<()> + Send + Sync>,

    /// This hook will be run before starting to watch for changes in files.
    /// By default it will add the backend crate directory and all its dependencies. But it
    /// excludes the target directory.
    pub backend_watch:
        Box<dyn Fn(&dyn ServeArgs, &mut RecommendedWatcher) -> Result<()> + Send + Sync>,

    /// This hook will be run before (re-)starting the backend.
    /// You can tweak the cargo command that is run here: adding/removing environment variables or
    /// adding arguments.
    /// By default it will do `cargo run -p <backend_crate>`.
    pub backend_command: Box<dyn Fn(&dyn ServeArgs, &mut Command) -> Result<()> + Send + Sync>,
}

impl Default for Hooks {
    fn default() -> Self {
        Self {
            backend_command: Box::new(|args, command| {
                command.args(&[
                    "run",
                    "-p",
                    &args
                        .build_args()
                        .backend_package()
                        .context("missing backend crate name")?
                        .name,
                ]);
                Ok(())
            }),
            backend_watch: Box::new(|args, watcher| {
                use notify::{RecursiveMode, Watcher};

                let metadata = args.build_args().metadata();
                let backend = args
                    .build_args()
                    .backend_package()
                    .context("missing backend crate name")?;
                let packages: HashMap<_, _> = metadata
                    .packages
                    .iter()
                    .map(|x| (x.name.as_str(), x))
                    .collect();
                let members: HashSet<_> = HashSet::from_iter(&metadata.workspace_members);

                backend
                    .dependencies
                    .iter()
                    .map(|x| packages.get(x.name.as_str()).unwrap())
                    .filter(|x| members.contains(&x.id))
                    .map(|x| x.manifest_path.parent().unwrap())
                    .chain(iter::once(backend.manifest_path.parent().unwrap()))
                    .try_for_each(|x| watcher.watch(x, RecursiveMode::Recursive))?;

                Ok(())
            }),
            frontend_watch: Box::new(|args, watcher| {
                use notify::{RecursiveMode, Watcher};

                let metadata = args.build_args().metadata();
                let frontend = args.build_args().frontend_package();
                let packages: HashMap<_, _> = metadata
                    .packages
                    .iter()
                    .map(|x| (x.name.as_str(), x))
                    .collect();
                let members: HashSet<_> = HashSet::from_iter(&metadata.workspace_members);

                frontend
                    .dependencies
                    .iter()
                    .map(|x| packages.get(x.name.as_str()).unwrap())
                    .filter(|x| members.contains(&x.id))
                    .map(|x| x.manifest_path.parent().unwrap())
                    .chain(iter::once(frontend.manifest_path.parent().unwrap()))
                    .try_for_each(|x| watcher.watch(x, RecursiveMode::Recursive))?;

                Ok(())
            }),
            pre_build: Box::new(|_, _, _| Ok(())),
            post_build: Box::new(
                |args, #[allow(unused_variables)] profile, wasm_js, wasm_bin| {
                    let build_path = args.build_path();
                    let wasm_js_path = build_path.join("app.js");
                    let wasm_bin_path = build_path.join("app_bg.wasm");

                    fs::write(&wasm_js_path, wasm_js).with_context(|| {
                        format!("could not write JS file to `{}`", wasm_js_path.display())
                    })?;
                    fs::write(&wasm_bin_path, wasm_bin).with_context(|| {
                        format!("could not write WASM file to `{}`", wasm_bin_path.display())
                    })?;

                    let index_path = build_path.join("index.html");
                    let static_dir = args
                        .frontend_package()
                        .manifest_path
                        .parent()
                        .unwrap()
                        .join("static");

                    if index_path.exists() {
                        fs::copy("index.html", &index_path).context(format!(
                            "could not copy index.html to `{}`",
                            index_path.display()
                        ))?;
                    } else if static_dir.exists() {
                        dir::copy(
                            &static_dir,
                            &build_path,
                            &dir::CopyOptions {
                                content_only: true,
                                ..dir::CopyOptions::new()
                            },
                        )
                        .with_context(|| {
                            format!(
                                "could not copy content of directory static: `{}` to `{}`",
                                static_dir.display(),
                                build_path.display()
                            )
                        })?;
                    } else {
                        fs::write(&index_path, DEFAULT_INDEX).with_context(|| {
                            format!(
                                "could not write default index.html to `{}`",
                                index_path.display()
                            )
                        })?;
                    }

                    #[cfg(feature = "sass")]
                    {
                        let options = args.sass_options(profile);
                        for style_path in args.sass_lookup_directories(profile) {
                            trace!("building SASS from {}", &style_path);
                            args.build_sass_from_dir(&style_path, options.clone())?;
                        }
                    }

                    Ok(())
                },
            ),
            #[cfg(feature = "dev-server")]
            serve: Box::new(|args, server| {
                use tide::{Body, Request, Response};

                let build_path = args.build_args().build_path().to_owned();
                let index_path = build_path.join("index.html");

                server.at("/").serve_dir(args.build_args().build_path())?;
                server.at("/").get(move |_| {
                    let index_path = index_path.clone();
                    async move { Ok(Response::from(Body::from_file(index_path).await?)) }
                });
                server.at("/*path").get(move |req: Request<()>| {
                    let build_path = build_path.clone();
                    async move {
                        match Body::from_file(build_path.join(req.param("path").unwrap())).await {
                            Ok(body) => Ok(Response::from(body)),
                            Err(_) => Ok(Response::from(
                                Body::from_file(build_path.join("index.html")).await?,
                            )),
                        }
                    }
                });

                Ok(())
            }),
        }
    }
}

fn build(mut profile: BuildProfile, args: &dyn BuildArgs, hooks: &Hooks) -> Result<()> {
    use wasm_bindgen_cli_support::Bindgen;

    info!("building frontend package");

    if args.profiling() {
        profile = BuildProfile::Profiling;
    }

    let frontend_package = args.frontend_package();

    let build_path = args.build_path();
    let _ = fs::remove_dir_all(build_path);
    fs::create_dir_all(build_path).with_context(|| {
        format!(
            "could not create build directory `{}`",
            build_path.display()
        )
    })?;

    let mut command = Command::new("cargo");

    command
        .args(&[
            "build",
            "--lib",
            "--target",
            "wasm32-unknown-unknown",
            "--manifest-path",
        ])
        .arg(&frontend_package.manifest_path)
        .args(match profile {
            BuildProfile::Profiling => &["--release"] as &[&str],
            BuildProfile::Release => &["--release"],
            BuildProfile::Dev => &[],
        });

    trace!("running pre-build hooks");
    (hooks.pre_build)(args, profile, &mut command)?;

    let status = command.status().context("could not start build process")?;

    if !status.success() {
        if let Some(code) = status.code() {
            bail!("build process exit with code {}", code);
        } else {
            bail!("build process has been terminated by a signal");
        }
    }

    let wasm_path = args
        .target_path()
        .join("wasm32-unknown-unknown")
        .join(match profile {
            BuildProfile::Profiling => "release",
            BuildProfile::Release => "release",
            BuildProfile::Dev => "debug",
        })
        .join(frontend_package.name.replace("-", "_"))
        .with_extension("wasm");

    let mut output = Bindgen::new()
        .input_path(wasm_path)
        .out_name("app")
        .web(true)
        .expect("fails only if multiple modes specified; qed")
        .debug(!matches!(profile, BuildProfile::Release))
        .generate_output()
        .context("could not generate WASM bindgen file")?;

    let wasm_js = output.js().to_owned();
    let wasm_bin = output.wasm_mut().emit_wasm();

    let wasm_bin = match profile {
        BuildProfile::Profiling => wasm_opt(wasm_bin, 0, 2, true, args.target_path())?,
        BuildProfile::Release => wasm_opt(wasm_bin, 1, 2, false, args.target_path())?,
        BuildProfile::Dev => wasm_bin,
    };

    trace!("running post-build hooks");
    (hooks.post_build)(args, profile, wasm_js, wasm_bin)?;

    Ok(())
}

#[cfg(feature = "dev-server")]
fn serve_frontend(
    args: &dyn ServeArgs,
    hooks: &Hooks,
) -> Result<Pin<Box<impl std::future::Future<Output = Result<()>> + Send + 'static>>> {
    use futures::TryFutureExt;

    if args.log() {
        tide::log::start();
    }
    let mut app = tide::new();

    (hooks.serve)(args, &mut app)?;

    info!(
        "Development server started: http://{}:{}",
        args.ip(),
        args.port()
    );

    Ok(Box::pin(
        app.listen(format!("{}:{}", args.ip(), args.port()))
            .map_err(Into::into),
    ))
}

#[cfg(not(feature = "dev-server"))]
fn watch_backend(args: &dyn ServeArgs, hooks: &Hooks) -> Result<()> {
    let (tx, rx) = mpsc::channel();

    let mut watcher: RecommendedWatcher = notify::Watcher::new(tx, time::Duration::from_secs(2))
        .context("could not initialize watcher")?;

    (hooks.backend_watch)(args, &mut watcher)?;

    struct BackgroundProcess(std::process::Child);

    impl Drop for BackgroundProcess {
        fn drop(&mut self) {
            // TODO: cleaner exit on Unix
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    let run_server = || -> Result<BackgroundProcess> {
        let mut command = Command::new("cargo");
        (hooks.backend_command)(args, &mut command)?;
        Ok(command.spawn().map(BackgroundProcess)?)
    };

    let mut process_guard = Some(run_server()?);

    watch_loop(args, rx, || {
        drop(process_guard.take());
        process_guard.replace(run_server()?);
        Ok(())
    });
}

fn watch_frontend(args: &dyn ServeArgs, hooks: &Hooks) -> Result<()> {
    let (tx, rx) = mpsc::channel();

    let mut watcher: RecommendedWatcher = notify::Watcher::new(tx, time::Duration::from_secs(2))
        .context("could not initialize watcher")?;

    (hooks.frontend_watch)(args, &mut watcher)?;

    let build_args = args.build_args();

    watch_loop(args, rx, || build(BuildProfile::Dev, build_args, hooks));
}

fn watch_loop(
    args: &dyn ServeArgs,
    rx: mpsc::Receiver<notify::DebouncedEvent>,
    mut callback: impl FnMut() -> Result<()>,
) -> ! {
    loop {
        use notify::DebouncedEvent::*;

        let message = rx.recv();
        match &message {
            Ok(Create(path)) | Ok(Write(path)) | Ok(Remove(path)) | Ok(Rename(_, path))
                if !path.starts_with(args.build_args().build_path())
                    && !path.starts_with(args.build_args().target_path())
                    && !path
                        .file_name()
                        .and_then(|x| x.to_str())
                        .map(|x| x.starts_with('.'))
                        .unwrap_or(false) =>
            {
                if let Err(err) = callback() {
                    error!("{}", err);
                }
            }
            Ok(_) => {}
            Err(e) => error!("watch error: {}", e),
        }
    }
}

#[allow(unused_variables, unreachable_code)]
fn wasm_opt(
    binary: Vec<u8>,
    shrink_level: u32,
    optimization_level: u32,
    debug_info: bool,
    target_path: impl AsRef<Path>,
) -> Result<Vec<u8>> {
    #[cfg(feature = "binaryen")]
    return match binaryen::Module::read(&binary) {
        Ok(mut module) => {
            module.optimize(&binaryen::CodegenConfig {
                shrink_level,
                optimization_level,
                debug_info,
            });
            Ok(module.write())
        }
        Err(()) => bail!("could not load WASM module"),
    };

    #[cfg(feature = "prebuilt-wasm-opt")]
    return {
        let wasm_opt = prebuilt_wasm_opt::install_wasm_opt(target_path)?;

        let mut command = Command::new(&wasm_opt);
        command
            .stderr(Stdio::inherit())
            .args(&["-o", "-", "-O"])
            .args(&["-ol", &optimization_level.to_string()])
            .args(&["-s", &shrink_level.to_string()]);
        if debug_info {
            command.arg("-g");
        }

        #[cfg(target_os = "macos")]
        {
            command.env("DYLD_LIBRARY_PATH", wasm_opt.parent().unwrap());
        }

        #[cfg(windows)]
        let delete_guard = {
            use std::io::Write;

            let mut binary = binary;
            let tmp = tempfile::NamedTempFile::new()?;
            tmp.as_file().write(&mut binary)?;
            command.arg(tmp.path());
            tmp
        };

        #[cfg(unix)]
        {
            use std::io::{Seek, SeekFrom, Write};

            let mut file = tempfile::tempfile()?;
            file.write_all(&binary)?;
            file.seek(SeekFrom::Start(0))?;
            command.stdin(file);
        }

        let output = command.output()?;
        if !output.status.success() {
            bail!("command `wasm-opt` failed.");
        }
        Ok(output.stdout)
    };

    warn!("no optimization has been done on the WASM");
    Ok(binary)
}

/// An extension for [`Package`] and for [`Metadata`] to run a cargo command a bit more easily.
/// Ideal for scripting.
pub trait PackageExt {
    /// Run the cargo command in the package's directory if ran on a [`Package`] or in the
    /// workspace root if ran on a [`Metadata`].
    fn cargo(&self, builder: impl FnOnce(&mut Command)) -> Result<CargoChild>;
}

impl PackageExt for Package {
    fn cargo(&self, builder: impl FnOnce(&mut Command)) -> Result<CargoChild> {
        let mut command = Command::new("cargo");
        command
            .current_dir(self.manifest_path.parent().unwrap())
            .stdout(Stdio::piped());

        builder(&mut command);

        Ok(CargoChild(command.spawn()?))
    }
}

impl PackageExt for Metadata {
    fn cargo(&self, builder: impl FnOnce(&mut Command)) -> Result<CargoChild> {
        let mut command = Command::new("cargo");
        command
            .current_dir(&self.workspace_root)
            .stdout(Stdio::piped());

        builder(&mut command);

        Ok(CargoChild(command.spawn()?))
    }
}

/// A cargo child process.
///
/// The child process is killed and waited if the instance is dropped.
pub struct CargoChild(Child);

impl CargoChild {
    /// Wait for the child process to finish and return an `Err(_)` if it didn't ended
    /// successfully.
    pub fn wait_success(&mut self) -> Result<()> {
        let status = self.0.wait()?;

        if let Some(code) = status.code() {
            if !status.success() {
                bail!("cargo exited with status: {}", code)
            }
        }

        if !status.success() {
            bail!("cargo exited with error")
        }

        Ok(())
    }

    /// Creates an iterator of Message from a Read outputting a stream of JSON messages. For usage
    /// information, look at the top-level documentation of [`cargo_metadata`].
    pub fn iter(&mut self) -> cargo_metadata::MessageIter<BufReader<ChildStdout>> {
        let reader = BufReader::new(self.0.stdout.take().unwrap());
        cargo_metadata::Message::parse_stream(reader)
    }
}

impl Drop for CargoChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// The wasmbl Prelude
///
/// The purpose of this module is to alleviate imports of many common types:
///
/// ```
/// # #![allow(unused_imports)]
/// use wasmbl::prelude::*;
/// ```
pub mod prelude {
    pub use wasmbl_proc_macro::*;

    pub use anyhow;
    #[cfg(feature = "dev-server")]
    pub use async_std;
    pub use cargo_metadata;
    pub use cargo_metadata::{Message, Metadata, Package};
    pub use fs_extra;
    #[cfg(feature = "dev-server")]
    pub use futures;
    pub use notify;
    pub use notify::RecommendedWatcher;
    #[cfg(feature = "sass")]
    pub use sass_rs;
    #[cfg(feature = "dev-server")]
    pub use tide;
    #[cfg(feature = "dev-server")]
    pub use tide::Server;

    pub use super::{
        BuildArgs, BuildProfile, CargoChild, DefaultBuildArgs, DefaultServeArgs, Hooks, PackageExt,
        ServeArgs,
    };
}
