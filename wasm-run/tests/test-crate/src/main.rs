use structopt::StructOpt;
use wasm_run::anyhow::Result;
use wasm_run::*;

#[wasm_run::main(
    other_cli_commands = run_other_cli_commands,
    prepare_build = prepare_build,
    post_build = post_build,
    serve = serve,
    watch = watch,
)]
#[derive(StructOpt, Debug)]
enum Cli {
    Build(BuildCommand),
    Serve(ServeCommand),
    Hello,
}

fn run_other_cli_commands(_: Cli) -> Result<()> {
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
    fn build_path(&self) -> &std::path::PathBuf {
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

fn prepare_build(args: &BuildCommand, _wasm_js: String) -> Result<()> {
    let _i = args.i;
    Ok(())
}

fn post_build(args: &BuildCommand) -> Result<()> {
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
