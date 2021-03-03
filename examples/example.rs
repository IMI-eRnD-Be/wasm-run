use structopt::StructOpt;

#[wasm_run::main("basic")]
#[derive(StructOpt, Debug)]
enum Cli {}
