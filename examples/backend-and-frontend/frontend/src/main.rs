use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use structopt::StructOpt;
use wasm_run::prelude::*;

#[wasm_run::main("frontend", run_server = run, other_cli_commands = other_cli_commands)]
#[derive(StructOpt, Debug)]
enum Cli {
    BuildContainerImage,
}

fn run(_args: DefaultServeArgs) -> anyhow::Result<()> {
    Err(backend::run().into())
}

fn other_cli_commands(cli: Cli, _metadata: &Metadata, _package: &Package) -> anyhow::Result<()> {
    match cli {
        Cli::Serve(_) | Cli::Build(_) | Cli::RunServer(_) => unreachable!(),
        Cli::BuildContainerImage => {
            println!("Building frontend...");

            let status = Command::new("cargo")
                .args(&["run", "--", "build"])
                .status()
                .unwrap();
            if !status.success() {
                anyhow::bail!("failed to compile frontend");
            }

            println!("Building backend...");

            let status = Command::new("cargo")
                .args(&[
                    "build",
                    "--release",
                    "-p",
                    "backend",
                    "--target",
                    "x86_64-unknown-linux-musl",
                ])
                .status()
                .unwrap();
            if !status.success() {
                anyhow::bail!("failed to compile backend");
            }

            println!("Building container image...");

            let dockerfile = Path::new("Dockerfile");
            let mut f = fs::File::create(&dockerfile)?;
            writeln!(f, "FROM gcr.io/distroless/cc")?;
            writeln!(
                f,
                "ADD target/x86_64-unknown-linux-musl/release/backend /backend"
            )?;
            writeln!(f, "ADD build /build")?;
            writeln!(f, "ENTRYPOINT [\"/backend\"]")?;
            drop(f);

            let status = Command::new("docker")
                .args(&["build", "-t", "wasm-run-example:latest", "."])
                .status()
                .unwrap();
            if !status.success() {
                anyhow::bail!("failed to build container image");
            }

            Ok(())
        }
    }
}
