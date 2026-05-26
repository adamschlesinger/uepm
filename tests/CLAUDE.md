# tests/ — Integration Test Guide

All integration tests use `mockito` for a fake npm registry and `tempfile::tempdir` for filesystem isolation. They run with `cargo test` (tokio runtime via `#[tokio::test]`).

## Test Files

| File | What it tests |
|---|---|
| `install_integration.rs` | Full install flow: registry fetch → tarball download → extraction → manifest/lockfile write |
| `publish_integration.rs` | Full publish flow: validation → tarball build → SHA computation → PUT to registry |

## Core Test Patterns

### Fake Registry with mockito
```rust
let mut server = Server::new_async().await;

// Mock the metadata endpoint
let meta_mock = server
    .mock("GET", "/@acme%2Fcool-plugin")
    .with_body(serde_json::to_string(&json!({ ... })).unwrap())
    .create_async()
    .await;

// Mock the tarball endpoint
let tarball_mock = server
    .mock("GET", "/tarball.tgz")
    .with_body(tarball_bytes)
    .create_async()
    .await;

// Build context pointing at the mock server — never read UEPM_REGISTRY env var
let ctx = UEPMContext::for_test(dir.path().to_path_buf(), &server.url(), None);
```

**Always use `UEPMContext::for_test`** in integration tests — `UEPMContext::new()` reads `UEPM_REGISTRY` from the environment and can race with parallel test threads.

### Filesystem Setup
```rust
let dir = tempdir().unwrap();
std::fs::create_dir(dir.path().join("Config")).unwrap();
std::fs::write(dir.path().join("Config/UEPM.ini"), "[Dependencies]\n").unwrap();
```

### Building a Fake Tarball
```rust
fn make_fake_tarball() -> Vec<u8> {
    use flate2::{write::GzEncoder, Compression};
    use tar::Builder;
    let enc = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = Builder::new(enc);
    // npm tarballs have a "package/" prefix — installer strips it
    builder.append_data(&mut header, "package/Plugin.uplugin", content).unwrap();
    builder.into_inner().unwrap().finish().unwrap()
}

fn sha512_integrity(data: &[u8]) -> String {
    format!("sha512-{}", general_purpose::STANDARD.encode(Sha512::digest(data)))
}
```

### Asserting Results
```rust
// Check file was extracted
assert!(dir.path().join("UEPMPlugins/cool-plugin/CoolPlugin.uplugin").exists());

// Check lockfile was written
let lock = LockFile::load(dir.path()).unwrap();
assert_eq!(lock.plugins["@acme/cool-plugin"].version, "1.0.0");

// Verify mocks were called
meta_mock.assert_async().await;
tarball_mock.assert_async().await;
```

## Adding a New Integration Test

1. Create `tests/<feature>_integration.rs`.
2. Use `UEPMContext::for_test` — never `UEPMContext::new()`.
3. One `tempdir` per test — never share temp dirs between tests.
4. Assert both the filesystem state AND the mock call counts (`.assert_async()`).
5. Run with `cargo test <feature>_integration` to validate in isolation.

## Unit Tests

Unit tests live inline in each module under `#[cfg(test)] mod tests { ... }`. They test pure logic (semver parsing, path manipulation, tarball building) without network or filesystem I/O.
