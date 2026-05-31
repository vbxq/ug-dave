# crates/ug-dave/vendor/

Third-party native dependencies that `ug-dave` builds against but does not vendor
into git (they are large and fully reproducible from a pinned commit). Keeping them
under the crate makes `ug-dave` self-contained (open-sourceable as a unit).

## libdave/  (Voice V4 DAVE E2EE)

`crates/ug-dave/vendor/libdave/` is a checkout of Discord's
[libdave](https://github.com/discord/libdave) (MLS / RFC 9420 group messaging for
voice end-to-end encryption), pinned to the commit audited in
`docs/audit/voice_dave_webrtc_boundary.md`. It is **gitignored** (see the root
`.gitignore`): only this README is committed.

It is the canonical, in-crate home for libdave -- the build finds it here with no
environment variable. Populate it once:

```sh
./scripts/build_libdave.sh
```

That clones + builds libdave + mlspp + the external-sender wrapper as static
archives under `crates/ug-dave/vendor/libdave/cpp/build/`:

- `crates/ug-dave/vendor/libdave/cpp/build/libdave.a`
- `crates/ug-dave/vendor/libdave/cpp/build/test/capi/external_sender.a`
- `crates/ug-dave/vendor/libdave/cpp/build/vcpkg_installed/<triplet>/lib/lib{mlspp,…,crypto}.a`

### How the build finds it

- `crates/ug-dave/build.rs` (feature `dave-ffi`) defaults `LIBDAVE_PREFIX` to
  `CARGO_MANIFEST_DIR/vendor/libdave/cpp` (i.e. this directory), so
  `cargo build -p voice --features dave-ffi` links it directly.
- `scripts/start.sh`, when `CELESTE_VOICE_DAVE_ENABLED=true`, defaults
  `LIBDAVE_PREFIX` to `crates/ug-dave/vendor/libdave/cpp` and rebuilds voice with
  `dave-ffi`.

Set `LIBDAVE_PREFIX` explicitly only to point at a libdave built somewhere else.
The default `dave-ffi`-off build never touches this directory (the `ug-dave` stub
links nothing native).
