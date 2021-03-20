use anyhow::{anyhow, Context, Result};
use rand::{thread_rng, Rng};
use std::{collections::HashMap, fs, path::PathBuf, thread, time::Duration};
use tera::Tera;
use walkdir::WalkDir;
use wasm_pack::command::build::{Build, BuildOptions};

/// Options passed to [`run()`] for bundling a web application.
pub struct WebBundlerOpt {
    /// Where to look for input files. Usually the root of the SPA crate.
    pub src_dir: PathBuf,
    /// The directory where output should be written to. In build.rs scripts, this should be read from the "OUT_DIR" environment variable.
    pub dist_dir: PathBuf,
    /// A directory that web-bundler can use to store temporary artifacts.
    pub tmp_dir: PathBuf,
    /// Passed into the index.html template as base_url. Example template usage: `<base href="{{ base_url }}">`
    pub base_url: Option<String>,
    /// Rename the webassembly bundle to include this version number.
    pub wasm_version: String,
    /// Build in release mode, instad of debug mode.
    pub release: bool,
    /// Path to the root of the workspace. A new target directory, called 'web-target' is placed there. If you aren't using a workspace, this can be wherever your `target` directory lives.
    pub workspace_root: PathBuf,
    /// Any additional directories that, if changes happen here, a rebuild is required.
    pub additional_watch_dirs: Vec<PathBuf>,
}

/// Bundles a web application for publishing
///
/// - This will run wasm-pack for the indicated crate.
/// - An index.html file will be read from the src_dir, and processed with the Tera templating engine.
/// - The .wasm file is versioned.
/// - Files in ./static are copied to the output without modification.
/// - If the file ./css/style.scss exists, it is compiled to CSS which can be inlined into the HTML template.
///
/// # Command Line Output
///
/// This function is intended to be called from a Cargo build.rs
/// script. It writes [Cargo
/// rerun-if-changed](https://doc.rust-lang.org/cargo/reference/build-scripts.html#outputs-of-the-build-script)
/// directives to stdout.
///
/// # Example index.html
///
/// ```html
/// <!DOCTYPE html>
/// <html lang="en">
///     <head>
///         <base href="{{ base_url }}">
///         <meta charset="utf-8">
///         <meta name="viewport" content="width=device-width, initial-scale=1, shrink-to-fit=no">
///
///         {{ stylesheet | safe }}
///
///         <title>My Amazing Website</title>
///     </head>
///     <body>
///         <div id="app"></div>
///         {{ javascript | safe }}
///     </body>
/// </html>
/// ```
///
/// # Thread Safety
///
/// This function sets and unsets environment variables, and so is not
/// safe to use in multithreaded build scripts.
///
/// It is safe to run multiple web-bundlers at the same time if they
/// are in different build.rs scripts, since Cargo runs each build.rs
/// script in its own process.
pub fn run(opt: WebBundlerOpt) -> Result<()> {
    list_cargo_rerun_if_changed_files(&opt)?;

    run_wasm_pack(&opt, 3)?;
    prepare_dist_directory(&opt)?;
    bundle_assets(&opt)?;
    bundle_js_snippets(&opt)?;
    bundle_index_html(&opt)?;
    bundle_app_wasm(&opt)?;
    Ok(())
}

fn list_cargo_rerun_if_changed_files(opt: &WebBundlerOpt) -> Result<()> {
    for entry in WalkDir::new(&opt.src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        println!("cargo:rerun-if-changed={}", entry.path().display());
    }
    for additional_watch_dir in &opt.additional_watch_dirs {
        for entry in WalkDir::new(&additional_watch_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }
    Ok(())
}

/// Clears any environment variables that Cargo has set for this build
/// script, so that they don't accidentally leak into build scripts
/// that run as part of the Wasm
/// build.
///
/// The list of variables to clear comes from here:
/// https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts
///
/// These variables are all reset to their original value after running.
///
/// Additional variables can be set if the caller wants to temporarily
/// change their value.
fn run_with_clean_build_script_environment_variables<T>(
    additional_vars: impl IntoIterator<Item = &'static str>,
    f: impl Fn() -> T,
) -> T {
    use std::ffi::OsString;

    let mut existing_values: HashMap<OsString, Option<OsString>> = HashMap::new();
    let build_script_vars_list = vec![
        "CARGO",
        "CARGO_MANIFEST_DIR",
        "CARGO_MANIFEST_LINKS",
        "CARGO_MAKEFLAGS",
        "OUT_DIR",
        "TARGET",
        "HOST",
        "NUM_JOBS",
        "OPT_LEVEL",
        "DEBUG",
        "PROFILE",
        "RUSTC",
        "RUSTDOC",
        "RUSTC_LINKER",
    ];

    let build_script_var_prefixes = vec!["CARGO_FEATURE_", "CARGO_CFG_", "DEP_"];

    for key in build_script_vars_list
        .into_iter()
        .chain(additional_vars.into_iter())
    {
        existing_values.insert(key.into(), std::env::var_os(key));
        std::env::remove_var(key);
    }

    for (key, value) in std::env::vars_os() {
        if build_script_var_prefixes
            .iter()
            .any(|prefix| key.to_string_lossy().starts_with(prefix))
        {
            existing_values.insert(key.clone(), Some(value));
            std::env::remove_var(key);
        }
    }

    let result = f();

    for (key, value) in existing_values {
        match value {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }
    result
}

fn run_wasm_pack(opt: &WebBundlerOpt, retries: u32) -> Result<()> {
    run_with_clean_build_script_environment_variables(vec!["CARGO_TARGET_DIR"], || {
        let target_dir = opt.workspace_root.join("web-target");

        std::env::set_var("CARGO_TARGET_DIR", target_dir.as_os_str());

        let build_opts = BuildOptions {
            path: Some(opt.src_dir.clone()),
            scope: None,
            mode: wasm_pack::install::InstallMode::Normal,
            disable_dts: true,
            target: wasm_pack::command::build::Target::Web,
            debug: !opt.release,
            dev: !opt.release,
            release: opt.release,
            profiling: false,
            out_dir: opt
                .tmp_dir
                .clone()
                .into_os_string()
                .into_string()
                .map_err(|_| anyhow!("couldn't parse tmp_dir into a String"))?,
            out_name: Some("package".to_owned()),
            extra_options: vec![],
        };

        let res = Build::try_from_opts(build_opts).and_then(|mut b| b.run());

        match res {
            Ok(_) => Ok(()),
            Err(e) => {
                let is_wasm_cache_error = e.to_string().contains("Error: Directory not empty")
                    || e.to_string().contains("binary does not exist");

                if is_wasm_cache_error && retries > 0 {
                    // This step could error because of a legitimate failure,
                    // or it could error because two parallel wasm-pack
                    // processes are conflicting over WASM_PACK_CACHE. This
                    // random wait in an attempt to get them restarting at
                    // different times.
                    let wait_ms = thread_rng().gen_range(1000..5000);
                    thread::sleep(Duration::from_millis(wait_ms));
                    run_wasm_pack(opt, retries - 1)
                } else {
                    Err(anyhow!(e))
                }
            }
        }
    })
}

fn prepare_dist_directory(opt: &WebBundlerOpt) -> Result<()> {
    if opt.dist_dir.is_dir() {
        fs::remove_dir_all(&opt.dist_dir).with_context(|| {
            format!(
                "Failed to clear old dist directory ({})",
                opt.dist_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&opt.dist_dir).with_context(|| {
        format!(
            "Failed to create the dist directory ({})",
            opt.dist_dir.display()
        )
    })?;
    Ok(())
}

fn bundle_assets(opt: &WebBundlerOpt) -> Result<()> {
    let src = opt.src_dir.join("static");
    let dest = &opt.dist_dir;
    if src.exists() {
        fs_extra::dir::copy(&src, &dest, &fs_extra::dir::CopyOptions::new()).with_context(
            || {
                format!(
                    "Failed to copy static files from {} to {}",
                    src.display(),
                    dest.display()
                )
            },
        )?;
    }
    Ok(())
}

fn bundle_index_html(opt: &WebBundlerOpt) -> Result<()> {
    let src_index_path = opt.src_dir.join("index.html");
    let index_html_template = fs::read_to_string(&src_index_path).with_context(|| {
        format!(
            "Failed to read {}. This should be a source code file checked into the repo.",
            src_index_path.display()
        )
    })?;

    let mut tera_context = tera::Context::new();

    let package_js_path = opt.tmp_dir.join("package.js");
    let package_js_content = fs::read_to_string(&package_js_path).with_context(|| {
        format!(
            "Failed to read {}. This should have been produced by wasm-pack",
            package_js_path.display()
        )
    })?;
    let javascript = format!(
        r#"<script type="module">{} init('app-{}.wasm'); </script>"#,
        package_js_content, opt.wasm_version
    );
    tera_context.insert("javascript", &javascript);

    tera_context.insert("base_url", opt.base_url.as_deref().unwrap_or("/"));

    let sass_options = sass_rs::Options {
        output_style: sass_rs::OutputStyle::Compressed,
        precision: 4,
        indented_syntax: true,
        include_paths: Vec::new(),
    };
    let style_src_path = opt.src_dir.join("css/style.scss");
    let style_css_content = sass_rs::compile_file(&style_src_path, sass_options)
        .map_err(|e| anyhow!("Sass compilation failed: {}", e))?;

    let stylesheet = format!("<style>{}</style>", style_css_content);
    tera_context.insert("stylesheet", &stylesheet);

    let index_html_content = Tera::one_off(&index_html_template, &tera_context, true)?;

    let dest_index_path = opt.dist_dir.join("index.html");
    fs::write(&dest_index_path, index_html_content).with_context(|| {
        format!(
            "Failed to write the index.html file to {}",
            dest_index_path.display()
        )
    })?;

    Ok(())
}

fn bundle_app_wasm(opt: &WebBundlerOpt) -> Result<()> {
    let src = opt.tmp_dir.join("package_bg.wasm");
    let dest = opt.dist_dir.join(format!("app-{}.wasm", opt.wasm_version));
    fs::copy(&src, &dest).with_context(|| {
        format!(
            "Failed to copy application wasm from {} to {}",
            src.display(),
            dest.display()
        )
    })?;
    Ok(())
}

fn bundle_js_snippets(opt: &WebBundlerOpt) -> Result<()> {
    let src = opt.tmp_dir.join("snippets");
    let dest = &opt.dist_dir;

    if src.exists() {
        fs_extra::dir::copy(&src, &dest, &fs_extra::dir::CopyOptions::new()).with_context(
            || {
                format!(
                    "Failed to copy js snippets from {} to {}",
                    src.display(),
                    dest.display()
                )
            },
        )?;
    }
    Ok(())
}
