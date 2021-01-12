use std::fs;
use std::path::Path;
use std::process::Command;

fn run_crate(path: &Path, args: &[&str]) {
    let output = Command::new("cargo")
        .current_dir(path)
        .args(&["run"])
        .arg("--")
        .args(args)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("stdout:\n{}\n", stdout);
    eprintln!("stderr:\n{}\n", stderr);
    assert!(
        output.status.success(),
        "build failed: {} args: {:?}",
        path.display(),
        args
    );
}

#[test]
fn run_test_crates() {
    let tests = Path::new("tests");

    let crate_path = tests.join("test-crate-name-vs-pkg-name");
    let build_path = Path::new("build");
    let _ = fs::remove_dir_all(build_path);
    run_crate(&crate_path, &["build"]);
    assert!(
        build_path.exists(),
        "test for `crate-name-vs-pkg-name` failed"
    );

    let crate_path = tests.join("test-cargo-helper");
    run_crate(&crate_path, &["test"]);

    let crate_path = tests.join("test-default-build-path");
    let build_path = crate_path.join("public");
    let _ = fs::remove_dir_all(&build_path);
    run_crate(&crate_path, &["build"]);
    assert!(build_path.exists(), "test for `default_build_path` failed");
    assert!(build_path.join("fancy.css").exists());

    let crate_path = tests.join("test-binaryen");
    let build_path = crate_path.join("build");
    let _ = fs::remove_dir_all(&build_path);
    run_crate(&crate_path, &["build"]);
    assert!(build_path.exists(), "test for `binaryen` failed");

    let crate_path = tests.join("test-no-serve");
    let build_path = crate_path.join("build");
    let _ = fs::remove_dir_all(&build_path);
    run_crate(&crate_path, &["build"]);
    assert!(build_path.exists(), "test for `no-serve` failed");

    let crate_path = tests.join("test-sass");
    let build_path = crate_path.join("build");
    let _ = fs::remove_dir_all(&build_path);
    run_crate(&crate_path, &["build"]);
    assert!(build_path.exists(), "test for `test-sass` failed");
    assert!(build_path.join("test1.css").exists());
    assert!(build_path.join("test2.css").exists());
    assert!(!build_path.join("_test3.css").exists());
    assert!(build_path.join("test4.css").exists());
    assert!(build_path.join("subdirectory").join("test5.css").exists());
    assert!(!build_path.join("subdirectory").join("_test6.css").exists());
    assert!(build_path.join("subdirectory").join("test7.css").exists());
}
