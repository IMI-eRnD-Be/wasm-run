use structopt::StructOpt;

#[wasmbl::main("frontend", "backend")]
#[derive(StructOpt, Debug)]
enum Cli {}
