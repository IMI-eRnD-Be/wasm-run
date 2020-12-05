fn build_crate(path: &std::path::Path) {
    let output = std::process::Command::new("cargo")
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

#[test]
fn examples() {
    let examples = std::path::PathBuf::from("examples");
    build_crate(&examples.join("basic"));
    build_crate(&examples.join("backend-and-frontend"));
}
