use anyhow::Result;
use std::path::PathBuf;
use structopt::StructOpt;
use wasm_run::prelude::*;

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

fn run_other_cli_commands(_: Cli, _metadata: &Metadata, _package: &Package) -> Result<()> {
    println!("Hello World!");
    Ok(())
}

#[derive(StructOpt, Debug)]
struct BuildCommand {
    #[structopt(skip)]
    i: i32,
}

#[derive(StructOpt, Debug)]
struct ServeCommand {
    #[structopt(flatten)]
    build_args: BuildCommand,

    #[structopt(skip)]
    j: i32,
}

impl BuildArgs for BuildCommand {
    fn build_path(&self) -> &PathBuf {
        todo!()
    }

    fn profiling(&self) -> bool {
        todo!()
    }
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

fn post_build(
    args: &BuildCommand,
    _profile: BuildProfile,
    _wasm_js: String,
    _wasm_bin: Vec<u8>,
) -> Result<()> {
    let _i = args.i;
    Ok(())
}

fn serve(args: &ServeCommand, _app: &mut tide::Server<()>) -> Result<()> {
    let _j = args.j;
    Ok(())
}

fn watch(args: &ServeCommand, _app: &mut notify::RecommendedWatcher) -> Result<()> {
    let _j = args.j;
    Ok(())
}

fn default_build_path(_metadata: &Metadata, _package: &Package) -> PathBuf {
    todo!()
}
