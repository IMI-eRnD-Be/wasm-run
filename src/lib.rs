//! # Synopsis
//!
//! Build tool that replaces `cargo run` to build WASM projects.
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

#![warn(missing_docs)]

#[cfg(feature = "prebuilt-wasm-opt")]
mod prebuilt_wasm_opt;

use anyhow::{bail, Context, Result};
use cargo_metadata::MetadataCommand;
use cargo_metadata::{Metadata, Package};
use downcast_rs::*;
use notify::RecommendedWatcher;
use once_cell::sync::OnceCell;
use std::fs;
use std::path::PathBuf;
#[cfg(feature = "serve")]
use std::pin::Pin;
use std::process::{Command, Stdio};
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

    /// Run the `build` command.
    fn run(self, hooks: Hooks) -> Result<()>
    where
        Self: Sized + 'static,
    {
        build(BuildProfile::Release, &self, &hooks)
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
    fn run(self, hooks: Hooks) -> Result<()>
    where
        Self: Sized + 'static,
    {
        // NOTE: the first step for serving is to call `build` a first time. The build directory
        //       must be present before we start watching files there.
        build(BuildProfile::Dev, self.build_args(), &hooks)?;
        #[cfg(feature = "serve")]
        {
            async_std::task::block_on(async {
                let t1 = async_std::task::spawn(serve(&self, &hooks)?);
                let t2 = async_std::task::spawn_blocking(move || watch(&self, &hooks));
                futures::try_join!(t1, t2)?;
                Ok(())
            })
        }
        #[cfg(not(feature = "serve"))]
        {
            watch(&self, &hooks)
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
pub struct Hooks {
    /// This hook will be run before the WASM is optimized.
    pub pre_build:
        Box<dyn Fn(&dyn BuildArgs, BuildProfile, &mut Command) -> Result<()> + Send + Sync>,

    /// This hook will be run after the WASM is optimized.
    #[allow(clippy::type_complexity)]
    pub post_build:
        Box<dyn Fn(&dyn BuildArgs, BuildProfile, String, Vec<u8>) -> Result<()> + Send + Sync>,

    /// This hook will be run before running the HTTP server.
    #[cfg(feature = "serve")]
    #[allow(clippy::type_complexity)]
    pub serve: Box<dyn Fn(&dyn ServeArgs, &mut Server<()>) -> Result<()> + Send + Sync>,

    /// This hook will be run before starting to watch for changes in files.
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
            post_build: Box::new(|args, _, wasm_js, wasm_bin| {
                let build_path = args.build_path();
                let index_path = build_path.join("index.html");

                if fs::copy("index.html", &index_path).is_err() {
                    fs::write(&index_path, DEFAULT_INDEX).with_context(|| {
                        format!(
                            "could not copy index.html nor write default to `{}`",
                            index_path.display()
                        )
                    })?;
                }
                fs::write(args.build_path().join("app.js"), wasm_js)?;
                fs::write(build_path.join("app_bg.wasm"), wasm_bin)
                    .context("could not write WASM file")?;

                Ok(())
            }),
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

    let build_path = args.build_path();
    let _ = fs::remove_dir_all(build_path);
    fs::create_dir_all(build_path).with_context(|| {
        format!(
            "could not create build directory `{}`",
            build_path.display()
        )
    })?;

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
                #[cfg(all(feature = "full-restart", unix, not(serve)))]
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
                #[cfg(not(all(feature = "full-restart", unix, not(serve))))]
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
        use std::io::{Seek, SeekFrom, Write};

        let mut binary = binary;
        let mut file = tempfile::tempfile()?;
        file.write(&mut binary)?;
        file.seek(SeekFrom::Start(0))?;
        let wasm_opt = prebuilt_wasm_opt::install_wasm_opt()?;
        let mut command = Command::new(wasm_opt);
        command
            .stdin(file)
            .stderr(Stdio::inherit())
            .args(&["-o", "-", "-O"])
            .args(&["-ol", &optimization_level.to_string()])
            .args(&["-s", &shrink_level.to_string()]);
        if debug_info {
            command.arg("-g");
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
    pub use cargo_metadata::{Metadata, Package};
    #[cfg(feature = "serve")]
    pub use futures;
    pub use notify;
    pub use notify::RecommendedWatcher;
    #[cfg(feature = "serve")]
    pub use tide;
    #[cfg(feature = "serve")]
    pub use tide::Server;

    pub use super::{
        BuildArgs, BuildProfile, DefaultBuildArgs, DefaultServeArgs, Hooks, ServeArgs,
    };
}
