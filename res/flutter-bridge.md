# Flutter bridge snapshot

`flutter-bridge.zip` contains the existing generated bridge from RustDesk's
[successful upstream build](https://github.com/rustdesk/rustdesk/actions/runs/42331919662),
artifact `13557254010`. It was generated with flutter_rust_bridge 1.80.1.

The upstream `src/flutter_ffi.rs` Git blob is
`0d6ca0ab5da47440a44ebfc441bcb5527d225715`, identical to this project's
RustDesk baseline before the Desk Rule implementation changes. Desk Rule
preserves those public function signatures.

The archive SHA-256 is
`b24fe8739d3d7bb74a3a167653cdda6acf0a6b65ae3e485421e248812627f3e8`.

Restore it from the repository root with
`python3 -m zipfile -e res/flutter-bridge.zip .`.
This restores ignored generated files without running bridge code generation.
