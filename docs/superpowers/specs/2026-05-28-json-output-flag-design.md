# `--json` Output Flag Design

**Date:** 2026-05-28
**Status:** Approved
**Context:** Phase 3 DX Polish. Every downstream Phase 3+ feature (search, outdated, info, Editor integration) depends on structured output. This flag is landed first so all subsequent commands can build on it.

---

## Goal

Add a global `--json` flag to all `uepm` commands. When present, each command emits a single JSON object or array on stdout instead of colored human-readable output. Errors also emit JSON on stdout. Exit codes are unchanged.

---

## Architecture

`--json` is a global clap flag on the root `Cli` struct. After parsing, `main.rs` sets `ctx.output_mode` via a builder method before dispatching commands. Each command reads `ctx.output_mode` to decide how to emit its result.

`OutputMode` is a two-variant enum added to `src/context.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode { #[default] Human, Json }
```

It is a field on `UEPMContext` with a default of `Human`. `main.rs` sets it after construction:

```rust
let mut ctx = UEPMContext::new()?;
ctx.output_mode = if cli.json { OutputMode::Json } else { OutputMode::Human };
```

`UEPMContext::for_test()` is **unchanged** — it defaults to `OutputMode::Human`, avoiding churn across all existing test call sites.

Each command collects its result into a typed struct deriving `serde::Serialize`, then calls a single shared helper:

```rust
// src/output.rs
pub fn emit_json<T: Serialize>(value: &T) {
    println!("{}", serde_json::to_string(value).expect("serialize"));
}
```

At the end of each `run()`:
```rust
if ctx.output_mode == OutputMode::Json {
    output::emit_json(&result);
} else {
    // existing print_* calls
}
```

Errors are handled in the single `if let Err(e) = result` block in `main.rs`. No changes needed inside individual command files for error paths.

---

## Changes by File

### `src/context.rs`
- Add `OutputMode` enum (`Human` default, `Json`). Add `pub output_mode: OutputMode` field to `UEPMContext`.
- `UEPMContext::new()`, `with_dir()`, and `for_test()` signatures are **unchanged** — they produce a context with `output_mode: OutputMode::Human`. `main.rs` sets the field directly after construction.

### `src/main.rs`
- Add `#[arg(long, global = true)] json: bool` to `Cli`.
- After construction: `ctx.output_mode = if cli.json { OutputMode::Json } else { OutputMode::Human };`
- **`--json` + `--yes` validation:** if `cli.json` and the command requires prompts (Init without `--yes`, Publish without `--yes`), emit `{"error": "..."}` and exit before running. Use clap's `requires` or a post-parse check — do not scatter the check into individual `run()` functions.
- Error handler: `if ctx.output_mode == OutputMode::Json { println!("{}", serde_json::json!({"error": e.to_string()})) } else { print_error(...) }`.

### `src/output.rs`
- Add `pub fn emit_json<T: Serialize>(value: &T)` — single `serde_json::to_string` + `println!`. All commands call this instead of inlining the JSON emit.

### `src/commands/list.rs`
- Add `#[derive(Serialize)]` to the existing `PluginEntry` struct — no new struct needed.
Emits `Vec<PluginEntry>`. `compatible` is already `Option<bool>` and serializes to `null` correctly.

### `src/commands/install.rs`
Output struct:
```rust
#[derive(Serialize)]
struct InstallResult {
    name: String,
    version: String,
    tarball: String,
    fresh: bool,
}
```
Emits an array of `InstallResult`. `fresh: true` means the package was **not in the lockfile before this invocation** (i.e., newly installed). `fresh: false` means it was already locked and skipped. Implementation: snapshot `lock.plugins.keys()` before resolution begins; diff against the final lockfile to determine freshness per package.

### `src/commands/uninstall.rs`
```rust
#[derive(Serialize)]
struct UninstallResult { removed: String }
```
Emits a single object.

### `src/commands/update.rs`
```rust
#[derive(Serialize)]
struct UpdateResult {
    name: String,
    from: Option<String>,
    to: String,
}
```
Emits an array, one entry per updated package. `from` is `null` if the package had no prior locked version.

### `src/commands/publish.rs`
- Add `#[derive(Serialize)]` to the existing `PackageMetadata` struct in `src/manifest.rs`.
Output:
```rust
#[derive(Serialize)]
struct PublishResult {
    name: String,     // from PackageMetadata
    version: String,  // from PackageMetadata
    registry: String, // from ctx.registry.base_url()
    dry_run: bool,
}
```
Emits a single object. `PackageMetadata` fields feed it directly.

### `src/commands/init.rs`
Init has two distinct code paths (project-context vs. plugin-context) producing structurally different data. Use a tagged enum:

```rust
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum InitOutput {
    Project {
        project_dir: String,
        engine_version: Option<String>,
        commit_plugins: bool,
    },
    Plugin {
        name: String,
        version: String,
        engine_range: String,
        vcs: Option<String>,
    },
}
```
Emits a single object. The `type` field is `"project"` or `"plugin"` so consumers can discriminate.

---

## Interactive Commands with `--json`

`init` and `publish` use interactive prompts. `--json` implies non-interactive. This is enforced **at clap parse time in `main.rs`**, not inside individual `run()` functions, so it can't be missed when new interactive commands are added.

```rust
// main.rs, after clap parse, before context construction
if cli.json {
    match &cli.command {
        Commands::Init { yes: false } | Commands::Publish { yes: false, .. } => {
            println!("{}", serde_json::json!({"error": "pass --yes when using --json"}));
            std::process::exit(1);
        }
        _ => {}
    }
}
```

- `uepm init --json --yes` works.
- `uepm publish --json --yes` works.
- `uepm init --json` (no `--yes`) prints `{"error": "pass --yes when using --json"}` and exits 1.

---

## JSON Shapes (Summary)

All shapes are stable contracts for the Editor plugin (Phase 4) and scripting consumers.

```
uepm list --json
→ [{"name":"@acme/foo","version":"1.2.3","engine_range":">=5.3,<6","compatible":true}, ...]

uepm install @acme/foo --json
→ [{"name":"@acme/foo","version":"1.2.3","tarball":"https://...","fresh":true}, ...]

uepm uninstall @acme/foo --json
→ {"removed":"@acme/foo"}

uepm update --json
→ [{"name":"@acme/foo","from":"1.0.0","to":"1.2.3"}, ...]

uepm publish --json --yes
→ {"name":"@acme/foo","version":"1.0.0","registry":"https://registry.npmjs.org","dry_run":false}

uepm init --json --yes
→ {"project_dir":"/path/to/project","engine_version":"5.3.0","commit_plugins":false}

# Error (any command, stdout, exit non-zero)
→ {"error":"package not found: @acme/missing"}
```

---

## Error Handling

`UepmError`'s `Display` impls (via `thiserror`) are used directly as the JSON error message — no new formatting needed. The existing single error handler in `main.rs` is the only place that changes.

---

## Testing

**Unit tests** (in each command file): existing tests keep `OutputMode::Human` and are unchanged.

**Integration tests** (`tests/`): each command's existing integration test grows `--json` variants. Tests spawn the binary via `assert_cmd::Command::cargo_bin("uepm")` with a temp dir + mockito fixture, capture stdout, call `serde_json::from_str`, and assert field values.

Note: existing integration tests use direct library calls (in-process). The `--json` tests use subprocess spawning via `assert_cmd` — a deliberate pattern difference since JSON output is only meaningful from the compiled binary's actual stdout.

`install` specifically needs at least 3 tests:
1. Single package — verifies object shape and `fresh: true`
2. Multiple packages — verifies output is an array, not a single object
3. Already-locked package — verifies `fresh: false`

---

## Dependencies

- `serde` with `derive` feature and `serde_json` — already present.
- `assert_cmd` — add to `[dev-dependencies]` for JSON integration tests (subprocess spawning).

---

## Out of Scope

- Streaming / ndjson output — deferred. Single-object-at-end is sufficient for Phase 3+4.
- `--json` on `uepm completions`, `uepm doctor`, `uepm outdated`, `uepm info`, `uepm search` — those commands are specced separately and will include `--json` from the start, following the pattern established here.
