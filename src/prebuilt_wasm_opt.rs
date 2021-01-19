use anyhow::{Context, Result};
use binary_install::Cache;
use std::path::{Path, PathBuf};

pub(crate) fn install_wasm_opt(target_path: impl AsRef<Path>) -> Result<PathBuf> {
    let cache = Cache::at(target_path.as_ref());

    let url = format!(
        "https://github.com/WebAssembly/binaryen/releases/download/version_{version}/binaryen-version_{version}-{arch}-{os}.tar.gz",
        version = "97",
        arch = platforms::TARGET_ARCH,
        os = platforms::TARGET_OS,
    );

    #[cfg(target_os = "macos")]
    let binaries = &["wasm-opt", "libbinaryen"];
    #[cfg(not(target_os = "macos"))]
    let binaries = &["wasm-opt"];

    eprintln!("Downloading wasm-opt...");
    Ok(cache
        .download(true, "wasm-opt", binaries, &url)
        .map_err(|err| err.compat())
        .with_context(|| format!("could not download binaryen: {}", url))?
        .expect("install is permitted; qed")
        .binary("wasm-opt")
        .map_err(|err| err.compat())?)
}
