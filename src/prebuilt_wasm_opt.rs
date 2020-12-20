use anyhow::{Context, Result};
use binary_install::Cache;
use std::path::PathBuf;

pub(crate) fn install_wasm_opt() -> Result<PathBuf> {
    let cache = Cache::new(env!("CARGO_PKG_NAME"))
        .map_err(|err| err.compat())
        .context("could not initialize cache")?;

    let url = format!(
        "https://github.com/WebAssembly/binaryen/releases/download/version_{version}/binaryen-version_{version}-{arch}-{os}.tar.gz",
        version = "97",
        arch = platforms::TARGET_ARCH,
        os = platforms::TARGET_OS,
    );

    #[cfg(target_os = "macos")]
    let binaries = &["wasm-opt", "libbinaryen.dylib"];
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
