use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use structopt::StructOpt;
use wasmbl::prelude::*;

#[wasmbl::main("frontend", "backend", other_cli_commands)]
#[derive(StructOpt, Debug)]
enum Cli {
    BuildContainerImage,
}

fn other_cli_commands(cli: Cli, metadata: &Metadata, _package: &Package) -> anyhow::Result<()> {
    match cli {
        Cli::BuildContainerImage => {
            println!("Building frontend...");
            Cli::build()?;

            println!("Building backend...");
            metadata
                .cargo(|command| {
                    command.args(&[
                        "build",
                        "--release",
                        "-p",
                        "backend",
                        "--target",
                        "x86_64-unknown-linux-musl",
                    ]);
                })?
                .wait_success()?;
            fs::copy(
                metadata
                    .target_directory
                    .join("x86_64-unknown-linux-musl")
                    .join("release")
                    .join("backend"),
                "backend-bin",
            )?;

            println!("Building container image...");

            let dockerfile = Path::new("Dockerfile");
            let mut f = fs::File::create(&dockerfile)?;
            writeln!(f, "FROM gcr.io/distroless/static")?;
            writeln!(f, "ADD backend-bin /backend")?;
            writeln!(f, "ADD build /build")?;
            writeln!(f, "ENTRYPOINT [\"/backend\"]")?;
            drop(f);

            let status = Command::new("docker")
                .args(&["build", "-t", "wasmbl-example:latest", "."])
                .status()
                .unwrap();
            if !status.success() {
                anyhow::bail!("failed to build container image");
            }

            Ok(())
        }
    }
}
