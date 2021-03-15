use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
        "run failed: {} args: {:?}",
        path.display(),
        args,
    );
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
fn run_test_crates() {
    let tests = Path::new("tests");

    {
        let crate_path = tests.join("test-crate-name-vs-pkg-name");
        let build_path = crate_path.join("build");
        let _ = fs::remove_dir_all(&build_path);
        run_crate(&crate_path, &["build"]);
        assert!(
            build_path.exists(),
            "test for `crate-name-vs-pkg-name` failed"
        );
    }

    {
        let crate_path = tests.join("test-cargo-helper");
        run_crate(&crate_path, &["test"]);
    }

    {
        let crate_path = tests.join("test-default-build-path");
        let build_path = crate_path.join("public");
        let _ = fs::remove_dir_all(&build_path);
        run_crate(&crate_path, &["build"]);
        assert!(build_path.exists(), "test for `default_build_path` failed");
        assert!(build_path.join("fancy.css").exists());
    }

    {
        let crate_path = tests.join("test-binaryen");
        let build_path = crate_path.join("build");
        let _ = fs::remove_dir_all(&build_path);
        run_crate(&crate_path, &["build"]);
        assert!(build_path.exists(), "test for `binaryen` failed");
    }

    {
        let crate_path = tests.join("test-no-serve");
        let build_path = crate_path.join("build");
        let _ = fs::remove_dir_all(&build_path);
        run_crate(&crate_path, &["build"]);
        assert!(build_path.exists(), "test for `no-serve` failed");
    }

    {
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

    {
        let crate_path = tests.join("test-bundler");
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
