fn test_crate(path: &str) {
    let output = std::process::Command::new("cargo")
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
    test_crate("tests/examples/basic");
    test_crate("tests/examples/backend-and-frontend");
}
