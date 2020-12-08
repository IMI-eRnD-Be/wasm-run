#![feature(proc_macro_hygiene, decl_macro)]

use structopt::StructOpt;
use wasm_run::prelude::*;
use rocket::response::NamedFile;
use std::path::{PathBuf, Path};

#[rocket::get("/")]
fn index() -> Option<NamedFile> {
    NamedFile::open(Path::new("build").join("index.html")).ok()
}

#[rocket::get("/<file..>")]
fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("build").join(file)).ok()
}

#[wasm_run::main("frontend", run_server = run)]
#[derive(StructOpt, Debug)]
enum Cli {}

fn run(_args: DefaultServeArgs) -> anyhow::Result<()> {
    Err(rocket::ignite().mount("/", rocket::routes![index, files]).launch().into())
}
