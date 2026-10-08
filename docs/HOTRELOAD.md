# Hot reload (dev only)

`arctis-host` (`src/bin/arctis-host.rs`) runs the app from
`libarctis_code.{dylib,so,dll}` instead of statically linked code, so
rebuilding the library picks up code changes. The release `ArctisCode`
binary is unaffected (static as before).

## Phase 0 (done): load once at startup

```sh
cargo build
./target/debug/arctis-host -- ./some/project
```

The host checks the `arctis_abi_version` symbol and refuses to run on
mismatch. Edit code → `cargo build` → restart the host.

ABI rules (both sides must match or the host exits with an error):

- Same toolchain and RUSTFLAGS for host and dylib.
- Same dependency versions (single `Cargo.lock`, one `cargo build`).
- Bump `HOT_ABI_VERSION` in `src/lib.rs` + `EXPECTED_ABI` in the host
  whenever the ABI surface changes.

## Phase 1 (next): live swap without restart

- Host holds a `HotApp` shim (`Box<dyn eframe::App>`) that forwards
  `update()` to the currently loaded dylib app.
- Watch the dylib mtime (reuse `notify` infra); on change: snapshot
  state → drop old app → reload library → restore snapshot.
- Requires `ArctisCodeApp` snapshot/restore (project root, tabs +
  dirty flags, tree expansion) and moving `NeovimBackend`/tokio
  ownership into the host so `nvim --embed` survives reload.

## Phase 2 (later): watch wiring + docs

- `cargo watch -x 'build -p arctis-code'` loop, reload debounced to
  idle moments (no save/copy in flight), fallback to full restart on
  any error.
