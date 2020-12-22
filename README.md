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
dependency to a binary of your project. Because of that you don't need any external tool, the
tooling is built as part of your dependences.

To build your project for production you can use the command `cargo run -- build` and to run a
development server that reloads automatically when the sources change you can use `cargo run --
serve`.

**Please note that there is a space between `--` and `build` and between `--` and `serve`!**

One of the main advantage of this library is that it provides greater customization: you can
set a few hooks during the build process in order to customize the build directory or use a
template to generate your index.html, download some CSS, ... you name it. I personally use it
to reduce the amount of files by bundling the CSS and the JS into the `index.html` so I had
only two files (`index.html`, `app_bg.wasm`).

## Examples

There are two basic examples to help you get started quickly:

 -  a ["basic"](https://github.com/IMI-eRnD-Be/wasm-run/tree/main/examples/basic) example for a
    frontend only app that rebuilds the app when a file change is detected;
 -  a ["backend-and-frontend"](https://github.com/IMI-eRnD-Be/wasm-run/tree/main/examples/backend-and-frontend)
    example using the web framework Rocket (backend) which uses Rocket itself to serve the file
    during the development (any file change is also detected and it rebuilds and restart
    automatically).

## Usage

All the details about the hooks can be find on the macro [`main`].

## Additional Information

 *  You can use this library to build examples in the `examples/` directory of your project.
    `cargo run --example your_example -- serve`. But you will need to specify the name of the
    WASM crate in your project and it must be present in the workspace. Please check the
    ["example"](https://github.com/IMI-eRnD-Be/wasm-run/blob/main/examples/example.rs) example.
 *  If you want to use your own backend you will need to disable the `serve` feature by
    disabling the default features. You can use the `full-restart` feature to force the backend
    to also be recompiled when a file changes (otherwise only the frontend is re-compiled). You
    will also need to specify `run_server` to the macro arguments to run your backend.
 *  You can add commands to the CLI by adding variants in the `enum`.
 *  You can add parameters to the `Build` and `Serve` commands by overriding them. Please check
    the documentation on the macro `main`.
 *  If you run `cargo run -- serve --profiling`, the WASM will be optimized.

License: MIT OR Apache-2.0
