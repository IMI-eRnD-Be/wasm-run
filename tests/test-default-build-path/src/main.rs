use std::path::PathBuf;
use structopt::StructOpt;
use wasmbl::prelude::*;

#[wasmbl::main(default_build_path)]
#[derive(StructOpt, Debug)]
enum Cli {}

fn default_build_path(_metadata: &Metadata, package: &Package) -> PathBuf {
    package.manifest_path.parent().unwrap().join("public")
}
