Backend and Frontend
====================

Development Server
------------------

This will run the backend in a subprocess and restart when changes are detected
in the files used by the backend.

The frontend will be rebuilt when changes are detected in the frontend files.

```
cargo run -- serve
```

You can now go to http://localhost:8000

Production Build
----------------

*Note:* This will build only the frontend. You must take care of the backend
build on your own.

```
cargo run -- build
```
