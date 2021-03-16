use cargo_metadata::MetadataCommand;
use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn clean(path: &Path) {
    let metadata = MetadataCommand::new().current_dir(&path).exec().unwrap();
    let members: HashSet<_> = metadata.workspace_members.iter().collect();
    for package in metadata.packages {
        if members.contains(&package.id) {
            let output = Command::new("cargo")
                .current_dir(&path)
                .args(&["clean", "-p"])
                .arg(package.name)
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            println!("stdout:\n{}\n", stdout);
            eprintln!("stderr:\n{}\n", stderr);
            assert!(output.status.success(), "clean failed: {}", path.display(),);
        }
    }
}

pub fn run_crate(path: &Path, run_args: &[&str]) {
    let mut args = vec!["run", "--"];
    args.extend(run_args);
    run_cargo(path, &args)
}

pub fn test_crate(path: &Path) {
    run_cargo(path, &["test"])
}

pub fn build_crate(path: &Path) -> Option<PathBuf> {
    use io::Read;

    clean(path);
    let mut child = Command::new("cargo")
        .current_dir(path)
        .args(&["build", "--message-format=json-render-diagnostics"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut out_dir = None;
    let reader = io::BufReader::new(child.stdout.take().unwrap());
    for message in cargo_metadata::Message::parse_stream(reader) {
        use cargo_metadata::Message;

        match message.unwrap() {
            Message::BuildScriptExecuted(script) => {
                out_dir = Some(script.out_dir);
            }
            _ => (),
        }
    }

    let status = child.wait().expect("Couldn't get cargo's exit status");
    let mut stderr = String::new();
    &child.stderr.unwrap().read_to_string(&mut stderr).unwrap();
    eprintln!("stderr:\n{}\n", stderr);

    assert!(status.success(), "build failed: {}", path.display(),);

    out_dir
}

pub fn run_cargo(path: &Path, args: &[&str]) {
    clean(path);
    let output = Command::new("cargo")
        // NOTE: this variable forces cargo to use the same toolchain but for the Rocket example
        //       we need nightly.
        .env_remove("RUSTUP_TOOLCHAIN")
        .current_dir(path)
        .args(args)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stdout:\n{}\n", stdout);
    eprintln!("stderr:\n{}\n", stderr);
    assert!(output.status.success());
}
