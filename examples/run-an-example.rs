//! This example demonstrates how to run an example using `cargo --example <name_of_the_example>`.
//!
//! In this repository you can run `cargo run --example run-an-example` and it will run this crate
//! which will run the frontend-only package available in the workspace of this repository under
//! `examples/frontend-only`.

use structopt::StructOpt;

// This will use the crate "frontend-only" in the workspace as frontend
#[wasmbl::main("frontend-only")]
#[derive(StructOpt, Debug)]
enum Cli {}
