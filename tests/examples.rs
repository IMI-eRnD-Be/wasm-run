use std::fs;
use std::path::Path;
use std::process::Command;

fn build_crate(path: &Path) {
    let output = Command::new("cargo")
        // NOTE: this variable forces cargo to use the same toolchain but for the Rocket example
        //       we need nightly.
        .env_remove("RUSTUP_TOOLCHAIN")
        .current_dir(path)
        .args(&["build"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stdout:\n{}\n", stdout);
    eprintln!("stderr:\n{}\n", stderr);
    assert!(output.status.success());
}

fn run_crate(path: &Path, args: &[&str]) {
    let output = Command::new("cargo")
        .args(&["run", "--manifest-path"])
        .arg(path.join("Cargo.toml"))
        .arg("--")
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
fn examples() {
    let examples = std::path::PathBuf::from("examples");
    build_crate(&examples.join("basic"));
    build_crate(&examples.join("backend-and-frontend"));

    let build_path = Path::new("build");
    let _ = fs::remove_dir_all(build_path);
    run_crate(&examples.join("test-crate-name-vs-pkg-name"), &["build"]);
    assert!(
        build_path.exists(),
        "test for `crate-name-vs-pkg-name` failed"
    );

    let build_path = examples.join("test-default-build-path").join("public");
    let _ = fs::remove_dir_all(&build_path);
    run_crate(&examples.join("test-default-build-path"), &["build"]);
    assert!(build_path.exists(), "test for `default_build_path` failed");
}
