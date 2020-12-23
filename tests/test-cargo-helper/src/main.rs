use structopt::StructOpt;
use wasm_run::prelude::*;

#[wasm_run::main(other_cli_commands)]
#[derive(StructOpt, Debug)]
enum Cli {
    Test,
}

fn other_cli_commands(cli: Cli, metadata: &Metadata, package: &Package) -> anyhow::Result<()> {
    match cli {
        Cli::Test => {
            let read_messages = |cargo: &mut CargoChild| {
                for message in cargo.iter() {
                    match message.unwrap() {
                        Message::CompilerMessage(msg) => {
                            println!("{:?}", msg);
                        }
                        Message::CompilerArtifact(artifact) => {
                            println!("{:?}", artifact);
                        }
                        Message::BuildScriptExecuted(script) => {
                            println!("{:?}", script);
                        }
                        Message::BuildFinished(finished) => {
                            println!("{:?}", finished);
                        }
                        _ => (), // Unknown message
                    }
                }
            };

            let mut cargo = package.cargo(|command| {
                command.args(&["build", "--message-format=json"]);
            })?;

            read_messages(&mut cargo);
            cargo.wait_success()?;

            let mut cargo = metadata.cargo(|command| {
                command.args(&["build", "--message-format=json"]);
            })?;

            read_messages(&mut cargo);
            cargo.wait_success()?;

            let build_path = Cli::build()?;

            if !build_path.exists() {
                anyhow::bail!("build path must exist");
            }

            std::fs::remove_dir_all(build_path)?;

            let build_path = Cli::build_with_args(&["--profiling"])?;

            if !build_path.exists() {
                anyhow::bail!("build path must exist");
            }

            Ok(())
        }
    }
}
