use structopt::StructOpt;

#[wasm_run::main(other_cli_commands = run_other_cli_commands)]
#[derive(StructOpt, Debug)]
enum Cli {
    Hello,
}

fn run_other_cli_commands(_: Cli) -> wasm_run::anyhow::Result<()> {
    println!("Hello World!");
    Ok(())
}
