#[test]
fn ui() {
    let output = std::process::Command::new("cargo")
        .env("RUSTFLAGS", "-Zmacro-backtrace")
        .args(&[
            "+nightly",
            "build",
            "--manifest-path",
            "tests/test-crate/Cargo.toml",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stdout:\n{}\n", stdout);
    eprintln!("stderr:\n{}\n", stderr);
    assert!(output.status.success());
}
