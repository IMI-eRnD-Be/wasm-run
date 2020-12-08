use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;
use wasm_run::prelude::*;

/// Makes an entrypoint to your binary.
#[wasm_run::main(
    other_cli_commands = run_other_cli_commands,
    post_build = post_build,
    serve = serve,
    watch = watch,
    default_build_path = default_build_path,
)]
#[derive(StructOpt, Debug)]
enum Cli {
    Build(BuildCommand),
    Serve(ServeCommand),
    Hello,
}

/// Define a custom `build` command.
#[derive(StructOpt, Debug)]
struct BuildCommand {
    #[structopt(skip)]
    i: i32,
}

impl BuildArgs for BuildCommand {
    fn build_path(&self) -> &PathBuf {
        self.default_build_path()
    }

    fn profiling(&self) -> bool {
        false
    }
}

/// Define a custom `serve` command.
#[derive(StructOpt, Debug)]
struct ServeCommand {
    #[structopt(flatten)]
    build_args: BuildCommand,

    #[structopt(skip)]
    j: i32,
}

impl ServeArgs for ServeCommand {
    fn build_args(&self) -> &dyn BuildArgs {
        &self.build_args
    }

    fn log(&self) -> bool {
        false
    }

    fn ip(&self) -> &str {
        "127.0.0.1"
    }

    fn port(&self) -> u16 {
        3000
    }
}

/// This function is called if you have added new commands to the enum.
fn run_other_cli_commands(cli: Cli, _metadata: &Metadata, _package: &Package) -> Result<()> {
    match cli {
        Cli::Build(_) | Cli::Serve(_) => unreachable!(),
        Cli::Hello => println!("Hello World!"),
    }

    Ok(())
}

/// This function is called after the build.
fn post_build(
    args: &BuildCommand,
    _profile: BuildProfile,
    wasm_js: String,
    wasm_bin: Vec<u8>,
) -> Result<()> {
    let _i = args.i;

    let build_path = args.build_path();
    fs::write(build_path.join("app.js"), wasm_js)?;
    fs::write(build_path.join("app_bg.wasm"), wasm_bin)?;
    fs::write(
        build_path.join("index.html"),
        "<html><body>Custom index.html</body>",
    )?;

    Ok(())
}

/// This function is called before serving files.
fn serve(args: &ServeCommand, server: &mut Server<()>) -> Result<()> {
    let _j = args.j;

    use tide::{Body, Response};

    let build_path = args.build_args().build_path();
    let index_path = build_path.join("index.html");

    server.at("/").serve_dir(args.build_args().build_path())?;
    server.at("/").get(move |_| {
        let index_path = index_path.clone();
        async move { Ok(Response::from(Body::from_file(index_path).await?)) }
    });

    Ok(())
}

/// This function is called when the watcher is being initialized.
fn watch(args: &ServeCommand, watcher: &mut RecommendedWatcher) -> Result<()> {
    let _j = args.j;

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
}

/// Define another build path if not provided by the user in the command-line arguments.
fn default_build_path(metadata: &Metadata, _package: &Package) -> PathBuf {
    metadata.workspace_root.join("build")
}
