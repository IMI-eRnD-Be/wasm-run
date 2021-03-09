Custom CLI Command
==================

You can add your own CLI commands. In this example, we added the command
`build-container-image` which makes a Docker image that builds the backend, the
frontend and pack the whole thing together in a container image using Docker.

Run:

```
cargo run -- build-container-image
```

Output:

```
   Compiling wasm-run v0.6.2-alpha.0 (/home/cecile/repos/wasm-run)
   Compiling backend v0.1.0 (/home/cecile/repos/wasm-run/examples/backend-and-frontend/backend)
   Compiling frontend v0.1.0 (/home/cecile/repos/wasm-run/examples/backend-and-frontend/frontend)
    Finished dev [unoptimized + debuginfo] target(s) in 10.43s
     Running `target/debug/frontend build-container-image`
Building frontend...
    Finished release [optimized] target(s) in 0.04s
Building backend...
   Compiling backend v0.1.0 (/home/cecile/repos/wasm-run/examples/backend-and-frontend/backend)
    Finished release [optimized] target(s) in 0.97s
Building container image...
Sending build context to Docker daemon  6.173MB
Step 1/4 : FROM gcr.io/distroless/static
 ---> b5f53c952b8e
Step 2/4 : ADD target/x86_64-unknown-linux-musl/release/backend /backend
 ---> Using cache
 ---> f094442bbf3b
Step 3/4 : ADD build /build
 ---> Using cache
 ---> bc350d9befcc
Step 4/4 : ENTRYPOINT ["/backend"]
 ---> Using cache
 ---> c704e0415e57
Successfully built c704e0415e57
Successfully tagged wasm-run-example:latest
```
