Frontend-Only
=============

Development Server
------------------

*Note:* there is an issue at the moment with cargo that forces you to add
`--bin <name_of_the_package>` even if there is only one package and only one
`bin`. https://github.com/rust-lang/cargo/issues/9235

```
cargo run --bin frontend-only -- serve
```

You can now go to http://localhost:3000

Production Build
----------------

*Note:* there is an issue at the moment with cargo that forces you to add
`--bin <name_of_the_package>` even if there is only one package and only one
`bin`. https://github.com/rust-lang/cargo/issues/9235

```
cargo run --bin frontend-only -- build
```
