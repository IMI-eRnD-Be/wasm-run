pub mod common;

use std::fs;
use std::path::Path;

use common::*;

#[test]
fn test_crates() {
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

    #[cfg(unix)]
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
