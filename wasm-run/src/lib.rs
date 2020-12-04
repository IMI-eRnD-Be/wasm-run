//! `wasm-run`

#![warn(missing_docs)]

use anyhow::{Context, Result};
use downcast_rs::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::pin::Pin;
use structopt::StructOpt;
pub use wasm_pack::command::build::BuildProfile;

pub use wasm_run_proc_macro::*;

pub use anyhow;
pub use async_std;
pub use futures;
pub use notify;
#[doc(hidden)]
pub use structopt;
pub use tide;

/// Build arguments.
#[derive(StructOpt, Debug)]
pub struct DefaultBuildArgs {
    /// Build directory output.
    #[structopt(long, default_value = "build")]
    pub build_path: PathBuf,
}

/// A trait that allow overriding the `build` command.
pub trait BuildArgs: Downcast {
    /// Build directory output.
    fn build_path(&self) -> &PathBuf;

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

/// A trait that allow overriding the `serve` command.
pub trait ServeArgs: Downcast + Send {
    /// Activate HTTP logs.
    fn log(&self) -> bool;

    /// IP address to bind.
    ///
    /// Use 0.0.0.0 to expose the server to your network.
    fn ip(&self) -> &str;

    /// Port number.
    fn port(&self) -> u16;

    /// Build arguments.
    fn build_args(&self) -> &dyn BuildArgs;

    /// Run the `serve` command.
    fn run(self, hooks: Hooks) -> Result<()>
    where
        Self: Sized + 'static,
    {
        async_std::task::block_on(async {
            build(BuildProfile::Dev, self.build_args(), &hooks)?;
            let t1 = async_std::task::spawn(serve(&self, &hooks)?);
            let t2 = async_std::task::spawn_blocking(move || watch(&self, &hooks));
            futures::try_join!(t1, t2)?;
            Ok(())
        })
    }
}

impl_downcast!(ServeArgs);

impl ServeArgs for DefaultServeArgs {
    fn log(&self) -> bool {
        self.log
    }

    fn ip(&self) -> &str {
        &self.ip
    }

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
    pub prepare_build: Box<dyn Fn(&dyn BuildArgs, String) -> Result<()> + Send + Sync>,

    /// This hook will be run after the WASM is optimized.
    pub post_build: Box<dyn Fn(&dyn BuildArgs) -> Result<()> + Send + Sync>,

    /// This hook will be run before running the HTTP server.
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

                for dir in &["src", "index.html"] {
                    watcher
                        .watch(dir, RecursiveMode::Recursive)
                        .with_context(|| format!("could not watch directory `{}`", dir))?;
                }

                Ok(())
            }),
            prepare_build: Box::new(|args, wasm_js| {
                fs::copy("index.html", args.build_path().join("index.html"))
                    .context("could not copy index.html")?;
                fs::write(args.build_path().join("app.js"), wasm_js)?;

                Ok(())
            }),
            post_build: Box::new(|_| Ok(())),
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

fn build(profile: BuildProfile, args: &dyn BuildArgs, hooks: &Hooks) -> Result<()> {
    use wasm_bindgen_cli_support::Bindgen;

    let cwd = env::current_dir().context("could not get current directory")?;
    wasm_pack::build::cargo_build_wasm(&cwd, profile, &[])
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

    let wasm_path = cwd
        .join("target")
        .join("wasm32-unknown-unknown")
        .join(match profile {
            BuildProfile::Dev => "debug",
            BuildProfile::Release => "release",
            _ => unimplemented!(),
        })
        .join(env!("CARGO_PKG_NAME"))
        .with_extension("wasm");
    let app_wasm_path = build_path.join("app_bg.wasm");

    let mut output = Bindgen::new()
        .input_path(wasm_path)
        .web(true)
        .expect("fails only if multiple modes specified; qed")
        .debug(!matches!(profile, BuildProfile::Release))
        .generate_output()
        .context("could not generate WASM bindgen file")?;

    let wasm_js = output.js().to_owned();
    fs::write(app_wasm_path, output.wasm_mut().emit_wasm()).context("could not write WASM file")?;

    (hooks.prepare_build)(args, wasm_js)?;

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

    (hooks.post_build)(args)?;

    Ok(())
}

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

fn watch(args: &dyn ServeArgs, hooks: &Hooks) -> Result<()> {
    use notify::{DebouncedEvent, RecommendedWatcher, Watcher};
    use std::sync::mpsc::channel;
    use std::time::Duration;

    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher =
        Watcher::new(tx, Duration::from_secs(2)).context("could not initialize watcher")?;

    (hooks.watch)(args, &mut watcher)?;

    let build_args = args.build_args();

    loop {
        use DebouncedEvent::*;

        match rx.recv() {
            Ok(Create(_)) | Ok(Write(_)) | Ok(Remove(_)) | Ok(Rename(_, _)) | Ok(Rescan) => {
                if let Err(err) = build(BuildProfile::Dev, build_args, hooks) {
                    eprintln!("{}", err);
                }
            }
            Ok(_) => {}
            Err(e) => eprintln!("watch error: {}", e),
        }
    }
}
