use structopt::StructOpt;
use wasm_run::prelude::*;

#[wasm_run::main(run_server = run_server)]
#[derive(StructOpt, Debug)]
enum Cli {}

fn run_server(_args: DefaultServeArgs) -> anyhow::Result<()> {
    todo!()
}
