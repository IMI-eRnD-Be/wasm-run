use structopt::StructOpt;

#[wasm_run::main]
#[derive(StructOpt, Debug)]
enum Cli {}
