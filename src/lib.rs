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
//!     `cargo run --example your_example -- serve`. But you will need to specific the name of the
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

use anyhow::{bail, Context, Result};
use cargo_metadata::MetadataCommand;
use downcast_rs::*;
use std::fs;
use std::path::PathBuf;
#[cfg(feature = "serve")]
use std::pin::Pin;
use structopt::StructOpt;
pub use wasm_pack::command::build::BuildProfile;

pub use wasm_run_proc_macro::*;

pub use anyhow;
#[cfg(feature = "serve")]
pub use async_std;
#[cfg(feature = "serve")]
pub use futures;
pub use notify;
#[doc(hidden)]
pub use structopt;
#[cfg(feature = "serve")]
pub use tide;

const DEFAULT_INDEX: &str = r#"<!DOCTYPE html><html><head><meta charset="utf-8"/><script type="module">import init from "/app.js";init();</script></head><body></body></html>"#;

/// Build arguments.
#[derive(StructOpt, Debug)]
pub struct DefaultBuildArgs {
    /// Build directory output.
    #[structopt(long, default_value = "build")]
    pub build_path: PathBuf,
}

/// A trait that allows overriding the `build` command.
pub trait BuildArgs: Downcast {
    /// Build directory output.
    fn build_path(&self) -> &PathBuf;

    /// Run the `build` command.
    fn run(self, crate_name: String, hooks: Hooks) -> Result<()>
    where
        Self: Sized + 'static,
    {
        build(BuildProfile::Release, &self, &crate_name, &hooks)
    }
}

impl_downcast!(BuildArgs);

impl BuildArgs for DefaultBuildArgs {
    fn build_path(&self) -> &PathBuf {
        &self.build_path
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
    fn run(self, crate_name: String, hooks: Hooks) -> Result<()>
    where
        Self: Sized + 'static,
    {
        build(BuildProfile::Dev, self.build_args(), &crate_name, &hooks)?;
        #[cfg(feature = "serve")]
        {
            async_std::task::block_on(async {
                let t1 = async_std::task::spawn(serve(&self, &hooks)?);
                let t2 = async_std::task::spawn_blocking(move || watch(&self, &crate_name, &hooks));
                futures::try_join!(t1, t2)?;
                Ok(())
            })
        }
        #[cfg(not(feature = "serve"))]
        {
            watch(&self, &crate_name, &hooks)
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
    /// This hook will be run before the WASM binary is optimized.
    #[allow(clippy::type_complexity)]
    pub prepare_build:
        Box<dyn Fn(&dyn BuildArgs, BuildProfile, String, Vec<u8>) -> Result<()> + Send + Sync>,

    /// This hook will be run after the WASM is optimized.
    pub post_build: Box<dyn Fn(&dyn BuildArgs, BuildProfile) -> Result<()> + Send + Sync>,

    /// This hook will be run before running the HTTP server.
    #[cfg(feature = "serve")]
    #[allow(clippy::type_complexity)]
    pub serve: Box<dyn Fn(&dyn ServeArgs, &mut tide::Server<()>) -> Result<()> + Send + Sync>,

    /// This hook will be run before starting to watch for changes in files.
    pub watch:
        Box<dyn Fn(&dyn ServeArgs, &mut notify::RecommendedWatcher) -> Result<()> + Send + Sync>,
}

impl Default for Hooks {
    fn default() -> Self {
        Self {
            watch: Box::new(|_, watcher| {
                use notify::{RecursiveMode, Watcher};
                use std::collections::HashSet;
                use std::iter::FromIterator;

                let metadata = MetadataCommand::new()
                    .exec()
                    .context("could not get cargo metadata")?;

                let _ = watcher.watch("index.html", RecursiveMode::Recursive);

                let members: HashSet<_> = HashSet::from_iter(metadata.workspace_members);

                for package in metadata.packages.iter().filter(|x| members.contains(&x.id)) {
                    let _ = watcher.watch(&package.manifest_path, RecursiveMode::Recursive);
                    let _ = watcher.watch(
                        package.manifest_path.parent().unwrap().join("src"),
                        RecursiveMode::Recursive,
                    );
                }

                Ok(())
            }),
            prepare_build: Box::new(|args, _, wasm_js, wasm_bin| {
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
            post_build: Box::new(|_, _| Ok(())),
            #[cfg(feature = "serve")]
            serve: Box::new(|args, app| {
                use tide::{Body, Response};

                let index_path = args.build_args().build_path().join("index.html");

                app.at("/").serve_dir(args.build_args().build_path())?;
                app.at("/").get(move |_| {
                    let index_path = index_path.clone();
                    async move { Ok(Response::from(Body::from_file(index_path).await?)) }
                });

                Ok(())
            }),
        }
    }
}

fn build(
    profile: BuildProfile,
    args: &dyn BuildArgs,
    crate_name: &str,
    hooks: &Hooks,
) -> Result<()> {
    use wasm_bindgen_cli_support::Bindgen;

    let metadata = MetadataCommand::new()
        .exec()
        .context("could not get cargo metadata")?;

    let crate_path = if let Some(package) = metadata.packages.iter().find(|x| x.name == crate_name)
    {
        package.manifest_path.parent().unwrap()
    } else {
        bail!(
            "Could not find crate named `{}` in the workspace",
            crate_name
        );
    };

    wasm_pack::build::cargo_build_wasm(crate_path, profile, &[])
        .map_err(|err| err.compat())
        .context("could not build WASM")?;

    let build_path = args.build_path();
    let _ = fs::remove_dir_all(build_path);
    fs::create_dir_all(build_path).with_context(|| {
        format!(
            "could not create build directory `{}`",
            build_path.display()
        )
    })?;

    let wasm_path = metadata
        .target_directory
        .join("wasm32-unknown-unknown")
        .join(match profile {
            BuildProfile::Dev => "debug",
            BuildProfile::Release => "release",
            _ => unimplemented!(),
        })
        .join(crate_name)
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

    (hooks.prepare_build)(args, profile, wasm_js, wasm_bin)?;

    if matches!(profile, BuildProfile::Release) {
        wasm_pack::wasm_opt::run(
            &wasm_pack::cache::get_wasm_pack_cache()
                .map_err(|err| err.compat())
                .context("could not get wasm-pack cache")?,
            build_path,
            &["-O".to_string()],
            true,
        )
        .map_err(|err| err.compat())
        .context("could not run wasm-opt")?; // TODO remove wasm-pack?
    }

    (hooks.post_build)(args, profile)?;

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

    Ok(Box::pin(
        app.listen(format!("{}:{}", args.ip(), args.port()))
            .map_err(Into::into),
    ))
}

fn watch(args: &dyn ServeArgs, crate_name: &str, hooks: &Hooks) -> Result<()> {
    use notify::{DebouncedEvent, RecommendedWatcher, Watcher};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher =
        Watcher::new(tx, Duration::from_secs(2)).context("could not initialize watcher")?;

    (hooks.watch)(args, &mut watcher)?;

    let build_args = args.build_args();

    #[cfg(not(feature = "serve"))]
    fn run_server() -> std::io::Result<impl std::any::Any> {
        use std::process::{Child, Command};

        struct ServerProcess(Child);

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

                    drop(crate_name);
                    drop(build_args);
                    drop(process_guard.take());

                    let err = std::process::Command::new("cargo")
                        .args(&["run", "--"])
                        .args(std::env::args_os().skip(1))
                        .exec();
                    eprintln!("could not restart process: {}", err);
                }
                #[cfg(not(all(feature = "full-restart", unix, not(serve))))]
                {
                    if let Err(err) = build(BuildProfile::Dev, build_args, &crate_name, hooks) {
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
