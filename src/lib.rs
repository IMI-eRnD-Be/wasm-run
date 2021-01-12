//! ![Rust](https://github.com/IMI-eRnD-Be/wasm-run/workflows/main/badge.svg)
//! [![Latest Version](https://img.shields.io/crates/v/wasm-run.svg)](https://crates.io/crates/wasm-run)
//! [![Docs.rs](https://docs.rs/wasm-run/badge.svg)](https://docs.rs/wasm-run)
//! [![LOC](https://tokei.rs/b1/github/IMI-eRnD-Be/wasm-run)](https://github.com/IMI-eRnD-Be/wasm-run)
//! [![Dependency Status](https://deps.rs/repo/github/IMI-eRnD-Be/wasm-run/status.svg)](https://deps.rs/repo/github/IMI-eRnD-Be/wasm-run)
//! ![License](https://img.shields.io/crates/l/wasm-run)
//!
//! # Synopsis
//!
//! Build tool that replaces `cargo run` to build WASM projects. Just like webpack, `wasm-run`
//! offers a great deal of customization.
//!
//! To build your WASM project you normally need an external tool like `wasm-bindgen`, `wasm-pack`
//! or `cargo-wasm`. `wasm-run` takes a different approach: it's a library that you install as a
//! dependency to a binary of your project. Because of that you don't need any external tool, the
//! tooling is built as part of your dependences.
//!
//! To build your project for production you can use the command `cargo run -- build` and to run a
//! development server that reloads automatically when the sources change you can use `cargo run --
//! serve`.
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
//! There are two basic examples to help you get started quickly:
//!
//!  -  a ["basic"](https://github.com/IMI-eRnD-Be/wasm-run/tree/main/examples/basic) example for a
//!     frontend only app that rebuilds the app when a file change is detected;
//!  -  a ["backend-and-frontend"](https://github.com/IMI-eRnD-Be/wasm-run/tree/main/examples/backend-and-frontend)
//!     example using the web framework Rocket (backend) which uses Rocket itself to serve the file
//!     during the development (any file change is also detected and it rebuilds and restart
//!     automatically).
//!
//! # Usage
//!
//! All the details about the hooks can be find on the macro [`main`].
//!
//! # Additional Information
//!
//!  *  You can use this library to build examples in the `examples/` directory of your project.
//!     `cargo run --example your_example -- serve`. But you will need to specify the name of the
//!     WASM crate in your project and it must be present in the workspace. Please check the
//!     ["example"](https://github.com/IMI-eRnD-Be/wasm-run/blob/main/examples/example.rs) example.
//!  *  If you want to use your own backend you will need to disable the `serve` feature by
//!     disabling the default features. You can use the `full-restart` feature to force the backend
//!     to also be recompiled when a file changes (otherwise only the frontend is re-compiled). You
//!     will also need to specify `run_server` to the macro arguments to run your backend.
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
//!     `sass-rs` is re-exported in the prelude of `wasm-run` for this purpose.
//!  *  `full-restart`: when this feature is active, the command is entirely restarted when changes
//!     are detected when serving files for development (`cargo run -- serve`). This is useful with
//!     custom `serve` command that uses a custom backend and if you need to detect changes in the
//!     backend code itself.

#![warn(missing_docs)]

#[cfg(feature = "prebuilt-wasm-opt")]
mod prebuilt_wasm_opt;

use anyhow::{bail, Context, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package};
use downcast_rs::*;
use fs_extra::dir;
use notify::RecommendedWatcher;
use once_cell::sync::OnceCell;
use std::fs;
use std::io::BufReader;
use std::path::PathBuf;
#[cfg(feature = "serve")]
use std::pin::Pin;
use std::process::{Child, ChildStdout, Command, Stdio};
use structopt::StructOpt;
#[cfg(feature = "serve")]
use tide::Server;

pub use wasm_run_proc_macro::*;

#[doc(hidden)]
pub use structopt;

const DEFAULT_INDEX: &str = r#"<!DOCTYPE html><html><head><meta charset="utf-8"/><script type="module">import init from "/app.js";init();</script></head><body></body></html>"#;

static METADATA: OnceCell<Metadata> = OnceCell::new();
static DEFAULT_BUILD_PATH: OnceCell<PathBuf> = OnceCell::new();
static PACKAGE: OnceCell<&Package> = OnceCell::new();
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
pub fn wasm_run_init(
    pkg_name: &str,
    default_build_path: Option<Box<dyn FnOnce(&Metadata, &Package) -> PathBuf>>,
    hooks: Hooks,
) -> Result<(&Metadata, &Package)> {
    let metadata = MetadataCommand::new()
        .exec()
        .context("this binary is not meant to be ran outside of its workspace")?;

    METADATA
        .set(metadata)
        .expect("the cell is initially empty; qed");

    let metadata = METADATA.get().unwrap();

    let package = METADATA
        .get()
        .unwrap()
        .packages
        .iter()
        .find(|x| x.name == pkg_name)
        .expect("the package existence has been checked during compile time; qed");

    PACKAGE
        .set(package)
        .expect("the cell is initially empty; qed");

    let package = PACKAGE.get().unwrap();

    DEFAULT_BUILD_PATH
        .set(if let Some(default_build_path) = default_build_path {
            default_build_path(metadata, package)
        } else {
            metadata.workspace_root.join("build")
        })
        .expect("the cell is initially empty; qed");

    if HOOKS.set(hooks).is_err() {
        panic!("the cell is initially empty; qed");
    }

    Ok((metadata, package))
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
    fn package(&self) -> &Package {
        PACKAGE
            .get()
            .expect("package has been initialized on startup; qed")
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
                    eprintln!(
                        "WARNING: could not walk into directory: `{}`",
                        input_dir.display()
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

        let package_path = self.package().manifest_path.parent().unwrap();

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
        let hooks = HOOKS.get().expect("we called wasm_run_init() first; qed");
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
    #[cfg(feature = "serve")]
    fn log(&self) -> bool;

    /// IP address to bind.
    ///
    /// Use 0.0.0.0 to expose the server to your network.
    #[cfg(feature = "serve")]
    fn ip(&self) -> &str;

    /// Port number.
    #[cfg(feature = "serve")]
    fn port(&self) -> u16;

    /// Build arguments.
    fn build_args(&self) -> &dyn BuildArgs;

    /// Run the `serve` command.
    fn run(self) -> Result<()>
    where
        Self: Sized + 'static,
    {
        let hooks = HOOKS.get().expect("we called wasm_run_init() first; qed");
        // NOTE: the first step for serving is to call `build` a first time. The build directory
        //       must be present before we start watching files there.
        build(BuildProfile::Dev, self.build_args(), hooks)?;
        #[cfg(feature = "serve")]
        {
            async_std::task::block_on(async {
                let t1 = async_std::task::spawn(serve(&self, hooks)?);
                let t2 = async_std::task::spawn_blocking(move || watch(&self, hooks));
                futures::try_join!(t1, t2)?;
                Ok(())
            })
        }
        #[cfg(not(feature = "serve"))]
        {
            watch(&self, hooks)
        }
    }
}

impl_downcast!(ServeArgs);

impl ServeArgs for DefaultServeArgs {
    #[cfg(feature = "serve")]
    fn log(&self) -> bool {
        self.log
    }

    #[cfg(feature = "serve")]
    fn ip(&self) -> &str {
        &self.ip
    }

    #[cfg(feature = "serve")]
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
    #[cfg(feature = "serve")]
    #[allow(clippy::type_complexity)]
    pub serve: Box<dyn Fn(&dyn ServeArgs, &mut Server<()>) -> Result<()> + Send + Sync>,

    /// This hook will be run before starting to watch for changes in files.
    /// By default it will add all the `src/` directories and `Cargo.toml` files of all the crates
    /// in the workspace plus the `static/` directory if it exists in the frontend crate.
    pub watch: Box<dyn Fn(&dyn ServeArgs, &mut RecommendedWatcher) -> Result<()> + Send + Sync>,
}

impl Default for Hooks {
    fn default() -> Self {
        Self {
            watch: Box::new(|args, watcher| {
                use notify::{RecursiveMode, Watcher};
                use std::collections::HashSet;
                use std::iter::FromIterator;

                let metadata = args.build_args().metadata();

                let _ = watcher.watch("index.html", RecursiveMode::Recursive);
                let _ = watcher.watch("static", RecursiveMode::Recursive);

                let members: HashSet<_> = HashSet::from_iter(&metadata.workspace_members);

                for package in metadata.packages.iter().filter(|x| members.contains(&x.id)) {
                    let _ = watcher.watch(&package.manifest_path, RecursiveMode::Recursive);
                    let _ = watcher.watch(
                        package.manifest_path.parent().unwrap().join("src"),
                        RecursiveMode::Recursive,
                    );
                }

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
                        .package()
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
                            args.build_sass_from_dir(&style_path, options.clone())?;
                        }
                    }

                    Ok(())
                },
            ),
            #[cfg(feature = "serve")]
            serve: Box::new(|args, server| {
                use tide::{Body, Response};

                let index_path = args.build_args().build_path().join("index.html");

                server.at("/").serve_dir(args.build_args().build_path())?;
                server.at("/").get(move |_| {
                    let index_path = index_path.clone();
                    async move { Ok(Response::from(Body::from_file(index_path).await?)) }
                });

                Ok(())
            }),
        }
    }
}

fn build(mut profile: BuildProfile, args: &dyn BuildArgs, hooks: &Hooks) -> Result<()> {
    use wasm_bindgen_cli_support::Bindgen;

    if args.profiling() {
        profile = BuildProfile::Profiling;
    }

    let package = args.package();

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
        .arg(&package.manifest_path)
        .args(match profile {
            BuildProfile::Profiling => &["--release"] as &[&str],
            BuildProfile::Release => &["--release"],
            BuildProfile::Dev => &[],
        });

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
        .metadata()
        .target_directory
        .join("wasm32-unknown-unknown")
        .join(match profile {
            BuildProfile::Profiling => "release",
            BuildProfile::Release => "release",
            BuildProfile::Dev => "debug",
        })
        .join(package.name.replace("-", "_"))
        .with_extension("wasm");

    let mut output = Bindgen::new()
        .input_path(wasm_path)
        .web(true)
        .expect("fails only if multiple modes specified; qed")
        .debug(!matches!(profile, BuildProfile::Release))
        .generate_output()
        .context("could not generate WASM bindgen file")?;

    let wasm_js = output.js().to_owned();
    let wasm_bin = output.wasm_mut().emit_wasm();

    let wasm_bin = match profile {
        BuildProfile::Profiling => wasm_opt(wasm_bin, 0, 2, true)?,
        BuildProfile::Release => wasm_opt(wasm_bin, 1, 2, false)?,
        BuildProfile::Dev => wasm_bin,
    };

    (hooks.post_build)(args, profile, wasm_js, wasm_bin)?;

    Ok(())
}

#[cfg(feature = "serve")]
fn serve(
    args: &dyn ServeArgs,
    hooks: &Hooks,
) -> Result<Pin<Box<impl std::future::Future<Output = Result<()>> + Send + 'static>>> {
    use futures::TryFutureExt;

    if args.log() {
        tide::log::start();
    }
    let mut app = tide::new();

    (hooks.serve)(args, &mut app)?;

    eprintln!(
        "Development server started: http://{}:{}",
        args.ip(),
        args.port()
    );

    Ok(Box::pin(
        app.listen(format!("{}:{}", args.ip(), args.port()))
            .map_err(Into::into),
    ))
}

fn watch(args: &dyn ServeArgs, hooks: &Hooks) -> Result<()> {
    use notify::{DebouncedEvent, Watcher};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher =
        Watcher::new(tx, Duration::from_secs(2)).context("could not initialize watcher")?;

    (hooks.watch)(args, &mut watcher)?;

    let build_args = args.build_args();

    #[cfg(not(feature = "serve"))]
    fn run_server() -> std::io::Result<impl std::any::Any> {
        struct ServerProcess(std::process::Child);

        impl Drop for ServerProcess {
            fn drop(&mut self) {
                // TODO: cleaner exit on Unix
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }

        let mut found = false;
        let args = std::env::args().skip(1).map(|x| {
            if !found && x == "serve" {
                found = true;
                "run-server".to_owned()
            } else {
                x
            }
        });

        Command::new(std::env::current_exe().unwrap())
            .args(args)
            .spawn()
            .map(ServerProcess)
    }

    #[cfg(not(feature = "serve"))]
    let mut process_guard = Some(run_server()?);

    loop {
        use DebouncedEvent::*;

        match rx.recv() {
            Ok(Create(_)) | Ok(Write(_)) | Ok(Remove(_)) | Ok(Rename(_, _)) | Ok(Rescan) => {
                #[cfg(all(feature = "full-restart", unix, not(feature = "serve")))]
                {
                    use std::os::unix::process::CommandExt;

                    drop(build_args);
                    drop(process_guard.take());

                    let err = Command::new("cargo")
                        .args(&["run", "--"])
                        .args(std::env::args_os().skip(1))
                        .exec();
                    eprintln!("could not restart process: {}", err);
                }
                #[cfg(not(all(feature = "full-restart", unix, not(feature = "serve"))))]
                {
                    if let Err(err) = build(BuildProfile::Dev, build_args, hooks) {
                        eprintln!("{}", err);
                    }
                    #[cfg(not(feature = "serve"))]
                    match run_server() {
                        Ok(guard) => drop(process_guard.replace(guard)),
                        Err(err) => eprintln!("running server error: {}", err),
                    }
                }
            }
            Ok(_) => {}
            Err(e) => eprintln!("watch error: {}", e),
        }
    }
}

#[allow(unused_variables, unreachable_code)]
fn wasm_opt(
    binary: Vec<u8>,
    shrink_level: u32,
    optimization_level: u32,
    debug_info: bool,
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
        let wasm_opt = prebuilt_wasm_opt::install_wasm_opt()?;

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

    eprintln!("WARNING: no optimization has been done on the WASM");
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

/// The wasm-run Prelude
///
/// The purpose of this module is to alleviate imports of many common types:
///
/// ```
/// # #![allow(unused_imports)]
/// use wasm_run::prelude::*;
/// ```
pub mod prelude {
    pub use wasm_run_proc_macro::*;

    pub use anyhow;
    #[cfg(feature = "serve")]
    pub use async_std;
    pub use cargo_metadata;
    pub use cargo_metadata::{Message, Metadata, Package};
    pub use fs_extra;
    #[cfg(feature = "serve")]
    pub use futures;
    pub use notify;
    pub use notify::RecommendedWatcher;
    #[cfg(feature = "sass")]
    pub use sass_rs;
    #[cfg(feature = "serve")]
    pub use tide;
    #[cfg(feature = "serve")]
    pub use tide::Server;

    pub use super::{
        BuildArgs, BuildProfile, CargoChild, DefaultBuildArgs, DefaultServeArgs, Hooks, PackageExt,
        ServeArgs,
    };
}
