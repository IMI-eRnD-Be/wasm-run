use structopt::StructOpt;
use wasm_run::*;

#[wasm_run::main]
#[derive(StructOpt, Debug)]
enum Cli {}
