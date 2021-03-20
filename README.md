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

 -  a ["basic"](https://github.com/IMI-eRnD-Be/wasm-run/tree/main/examples/basic) example for a
    frontend only app that rebuilds the app when a file change is detected;
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

Web Bundler
===========

[![crates.io](https://img.shields.io/crates/v/web-bundler.svg)](https://crates.io/crates/web-bundler)
[![build status](https://panoptix.semaphoreci.com/badges/web-bundler/branches/main.svg)](https://panoptix.semaphoreci.com/projects/web-bundler)

Manages the building of WebAssembly single page app frontends from a
build.rs script so that they can easily be embedded into Rust API
projects.

Internally, the bundler uses
[wasm-pack](https://github.com/rustwasm/wasm-pack) to do the actual
webassembly compilation.

## Prerequisites

From a clean Rustup-based Rust installation, there are no additional
steps. Web Bundler will download and install any dependencies it
needs.

For Rust installations that are not done with Rustup, you will need to
add the `wasm32-unknown-unknown` compilation target manually (see the
[wasm-pack docs](https://rustwasm.github.io/docs/wasm-pack/prerequisites/non-rustup-setups.html)
for details on how to do this).

## Running the Demo Example

There is an example usage in the [example directory](./example). To
run the example, open a terminal in the example directory and run
`cargo run`. Then, open a web browser and navigate to
<http://localhost:3030/>. You should see a Seed web application.

## Usage

Web-bundler expects you to have two projects: a frontend project using
a single page app framework (like [Seed]), and a backend project using a
web server framework (like [warp]).
These projects should be in a common
workspace.

### Changes to your frontend project

1. Update your index.html to allow templating in Javascript and CSS.

   Specifically, you need to add `{{ stylesheet | safe }}` to the
   `<head>` section, and `{{ javascript | safe }}` to the end of the
   `<body>`. Optionally, if you want to set the base url, add `<base
   href="{{ base_url }}">` to the `<head>` as well.

   See the example [frontend index.html](./example/frontend/index.html).

2. Create a root stylesheet for your app called `./css/style.scss`.

   This stylesheet will be compiled to CSS, and embedded directly into
   your index.html file.

   See the example [frontend style.scss](./example/frontend/css/style.scss).

3. Put all of your static assets in the `static` directory

   All files in the static directory will be copied directly to a static
   folder in the output directory.

   See the example [frontend static directory](./example/frontend/static/).

### Changes to your API project

1. Update your Cargo.toml to depend on your frontend project and web-bundler

   We depend on the frontend project in Cargo.toml so that Cargo knows to
   rerun build.rs whenever the frontend project changes.

   See the example [backend Cargo.toml](./example/backend/Cargo.toml).

2. Add a build.rs script that calls web-bundler for your frontend

   See the example [backend build.rs](./example/backend/build.rs).

3. Use [Rust Embed](https://lib.rs/crates/rust-embed) to embed your built frontend into your API binary

   See the example [backend main.rs](./example/backend/main.rs). Our
   example uses the [warp web server][warp]. Rust Embed also has examples for
   other web servers in [their repo](https://github.com/pyros2097/rust-embed/tree/master/examples).

## target and web-target directories

When web-bundler compiles the frontend, it overrides the default
target directory to be `web-target` instead of `target`. This is done
because, if the backend and frontend are in the same workspace, Cargo
will already be locking `target` while running the build.rs
script.

[warp]: https://github.com/seanmonstar/warp
[Seed]: https://github.com/seed-rs/seed

#### License

<sup>
Licensed under either of
<a href="LICENSE-APACHE">Apache License, Version 2.0</a>
or
<a href="LICENSE-MIT">MIT license</a>
at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
