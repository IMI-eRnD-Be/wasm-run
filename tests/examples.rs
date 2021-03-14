use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn run_cargo(path: &Path, args: &[&str]) {
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

fn test_crate(path: &Path) {
    let output = Command::new("cargo")
        .current_dir(path)
        .args(&["test"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stdout:\n{}\n", stdout);
    eprintln!("stderr:\n{}\n", stderr);
    assert!(output.status.success(), "test failed: {}", path.display(),);
}

fn build_crate(path: &Path) -> Option<PathBuf> {
    use io::Read;

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

#[test]
fn build_example_crates() {
    let examples = Path::new("examples");
    run_cargo(
        &examples,
        &["run", "--example", "run-an-example", "--", "build"],
    );
    run_cargo(
        &examples.join("frontend-only"),
        &["build", "--bin", "frontend-only"],
    );
    run_cargo(&examples.join("backend-and-frontend"), &["build"]);
    #[cfg(target_os = "linux")]
    run_cargo(
        &examples.join("custom-cli-command"),
        &["run", "-p", "run", "--", "build-container-image"],
    );

    {
        let crate_path = examples.join("bundler");
        //let build_path = crate_path.join("public");
        // ./target/debug/build/backend-b938b1376b56fe6c/out/ui
        //let _ = fs::remove_dir_all(&build_path);
        let out_dir = build_crate(&crate_path);
        let out_dir = out_dir.unwrap();
        assert!(out_dir.join("ui").join("app-0.0.0.wasm").exists());
        assert!(out_dir.join("ui").join("index.html").exists());
        test_crate(&crate_path);
    }
}
