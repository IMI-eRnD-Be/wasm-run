#![feature(proc_macro_hygiene, decl_macro)]

use rocket::response::NamedFile;
use std::path::{Path, PathBuf};

#[rocket::get("/")]
pub fn index() -> Option<NamedFile> {
    NamedFile::open(Path::new("build").join("index.html")).ok()
}

#[rocket::get("/<file..>")]
pub fn files(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("build").join(file)).ok()
}

fn main() {
    rocket::ignite()
        .mount("/", rocket::routes![index, files])
        .launch();
}
