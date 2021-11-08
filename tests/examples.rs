use std::path::Path;
use std::process::Command;

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

#[test]
fn build_example_crates() {
    let examples = Path::new("examples");
    run_cargo(
        examples,
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
}
