use structopt::StructOpt;
use wasm_run::prelude::*;

#[wasm_run::main()]
#[derive(StructOpt, Debug)]
enum Cli {}
