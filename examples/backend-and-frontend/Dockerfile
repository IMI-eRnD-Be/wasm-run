FROM gcr.io/distroless/cc
ADD target/x86_64-unknown-linux-musl/release/backend /backend
ADD build /build
ENTRYPOINT ["/backend"]
