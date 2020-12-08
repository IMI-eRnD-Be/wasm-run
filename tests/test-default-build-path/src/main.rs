use std::path::PathBuf;
use structopt::StructOpt;
use wasm_run::*;

#[wasm_run::main(default_build_path = foo)]
#[derive(StructOpt, Debug)]
enum Cli {}

fn foo(_metadata: &Metadata, package: &Package) -> PathBuf {
    package.manifest_path.parent().unwrap().join("public")
}
