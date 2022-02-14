**This repository is now deprecated in favor of
[xtask-wasm](https://github.com/rustminded/xtask-wasm).
Only patch fixes will be accepted.**

# wasm-run

![Rust](https://github.com/IMI-eRnD-Be/wasm-run/workflows/main/badge.svg)
[![Latest Version](https://img.shields.io/crates/v/wasm-run.svg)](https://crates.io/crates/wasm-run)
[![Docs.rs](https://docs.rs/wasm-run/badge.svg)](https://docs.rs/wasm-run)
[![LOC](https://tokei.rs/b1/github/IMI-eRnD-Be/wasm-run)](https://github.com/IMI-eRnD-Be/wasm-run)
[![Dependency Status](https://deps.rs/repo/github/IMI-eRnD-Be/wasm-run/status.svg)](https://deps.rs/repo/github/IMI-eRnD-Be/wasm-run)
![License](https://img.shields.io/crates/l/wasm-run)

## Synopsis

Build tool that replaces `cargo run` to build WASM projects. Just like webpack, `wasm-run`
offers a great deal of customization.

To build your WASM project you normally need an external tool like `wasm-bindgen`, `wasm-pack`
or `cargo-wasm`. `wasm-run` takes a different approach: it's a library that you install as a
dependency to your project. Because of that you don't need any external tool, the
tooling is built as part of your dependencies, which makes the CI easier to set up and reduce
the hassle for new comers to start working on the project.

To build your project for production you can use the command `cargo run -- build`. You can also
run a development server that rebuilds automatically when the code changes:
`cargo run -- serve`. It doesn't rebuild everything, only the backend if the backend changed or
the frontend if the frontend changed.

**Please note that there is a space between `--` and `build` and between `--` and `serve`!**

One of the main advantage of this library is that it provides greater customization: you can
set a few hooks during the build process in order to customize the build directory or use a
template to generate your index.html, download some CSS, ... you name it. I personally use it
to reduce the amount of files by bundling the CSS and the JS into the `index.html` so I had
only two files (`index.html`, `app_bg.wasm`).

## Examples

There are 3 basic examples to help you get started quickly:

 -  a ["frontend-only"](https://github.com/IMI-eRnD-Be/wasm-run/tree/main/examples/frontend-only)
    example for a frontend only app that rebuilds the app when a file change is detected;
 -  a ["backend-and-frontend"](https://github.com/IMI-eRnD-Be/wasm-run/tree/main/examples/backend-and-frontend)
    example using the web framework Rocket (backend) which uses Rocket itself to serve the file
    during the development (any file change is also detected and it rebuilds and restart
    automatically).
 -  a ["custom-cli-command"](https://github.com/IMI-eRnD-Be/wasm-run/tree/main/examples/custom-cli-command)
    example that adds a custom CLI command named `build-docker-image` which build the backend,
    the frontend and package the whole thing in a container image.

## Usage

All the details about the hooks can be found on the macro [`main`].

## Additional Information

 *  You can use this library to build examples in the `examples/` directory of your project.
    `cargo run --example your_example -- serve`. But you will need to specify the name of the
    WASM crate in your project and it must be present in the workspace. Please check the
    ["run-an-example"](https://github.com/IMI-eRnD-Be/wasm-run/blob/main/examples/run-an-example.rs)
    example.
 *  If you want to use your own backend you will need to disable the `dev-server` feature
    by disabling the default features. You can use the `full-restart` feature to force the
    backend to also be recompiled when a file changes (otherwise only the frontend is
    re-compiled). You will also need to specify `run_server` to the macro arguments to run your
    backend.
 *  You can add commands to the CLI by adding variants in the `enum`.
 *  You can add parameters to the `Build` and `Serve` commands by overriding them. Please check
    the documentation on the macro `main`.
 *  If you run `cargo run -- serve --profiling`, the WASM will be optimized.

## Features

 *  `prebuilt-wasm-opt`: if you disable the default features and enable this feature, a binary
    of wasm-opt will be downloaded from GitHub and used to optimize the WASM. By default,
    wasm-opt is compiled among the dependencies (`binaryen`). This is useful if you run into
    troubles for building `binaryen-sys`. (`binaryen` cannot be built on Netlify at the
    moment.)
 *  `sass`: support for SASS and SCSS. All SASS and SCSS files found in the directories
    `styles/`, `assets/`, `sass/` and `css/` will be automatically transpiled to CSS and placed
    in the build directory. This can be configured by overriding:
    [`BuildArgs::build_sass_from_dir`], [`BuildArgs::sass_lookup_directories`],
    [`BuildArgs::sass_options`] or completely overriden in the [`Hooks::post_build`] hook.
    `sass-rs` is re-exported in the prelude of `wasm-run` for this purpose.
 *  `full-restart`: when this feature is active, the command is entirely restarted when changes
    are detected when serving files for development (`cargo run -- serve`). This is useful with
    custom `serve` command that uses a custom backend and if you need to detect changes in the
    backend code itself.

License: MIT OR Apache-2.0
