pub mod common;

use std::path::Path;

use common::*;

#[test]
fn examples() {
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
