use structopt::StructOpt;

#[wasm_run::main("frontend", "backend")]
#[derive(StructOpt, Debug)]
enum Cli {}
