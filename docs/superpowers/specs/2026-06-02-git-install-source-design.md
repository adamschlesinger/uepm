# `git:` Install Source Design

**Date:** 2026-06-02  
**Status:** Draft  
**Context:** Phase 3 DX Polish. Complements the existing `file:` local-path source with support for installing plugins directly from a Git repository URL, without requiring the plugin to be published to any registry.

---

## Goal

Allow any `git:…` URL to be used as a version range in `Config/UEPM.ini` and as an inline argument to `uepm install`. After cloning, the resolved commit SHA is pinned in `uepm.lock` so subsequent installs are fully reproducible without re-hitting the network. `uepm update` re-clones from the named ref and updates the lock if the SHA has changed.

---

## Syntax

The `git:` prefix followed by any Git-remote URL. An optional `#<ref>` fragment specifies a branch name, tag, or full 40-character commit SHA. Omitting `#` is equivalent to `#HEAD` (the remote's default branch).

```
# HTTPS public repo
uepm install git:https://github.com/org/repo

# HTTPS with branch
uepm install git:https://github.com/org/repo#main

# HTTPS with tag
uepm install git:https://github.com/org/repo#v1.4.0

# HTTPS with pinned SHA
uepm install git:https://github.com/org/repo#a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2

# SSH (uses system SSH agent / ~/.ssh keys)
uepm install git:git@github.com:org/repo#main
```

In `Config/UEPM.ini` the spec is stored verbatim as the version range:

```toml
[Dependencies]
"@acme/cool-plugin" = "git:https://github.com/acme/cool-plugin#main"
"@studio/vfx-core"  = "git:git@github.com:studio/vfx-core#v2.1.0"
```

### Package name for bare `git:` installs

When `uepm install git:…` is used without a `@scope/name` prefix, UEPM derives the package name from the URL: strip any trailing `.git`, take the last path segment. For example, `github.com/acme/cool-plugin` → `cool-plugin`. To use a scoped name, pass it explicitly with `=`:

```
uepm install @acme/cool-plugin=git:https://github.com/acme/cool-plugin#main
```

The `=`-separated form is introduced with this feature. The `@`-at-sign form already used by semver ranges (`@acme/pkg@1.0.0`) is unambiguous because a `git:` URL never starts with a version character.

---

## Architecture

`git:` detection and installation follows the same branch structure as `file:` in `resolve_and_install`. A new `clone_git` function in `src/installer.rs` handles the actual subprocess work. No new crates are required — `git` is invoked via `tokio::process::Command`, which is already present.

### `clone_git` (src/installer.rs)

```rust
pub async fn clone_git(
    git_spec: &str,        // full "git:…" string from the range field
    package_name: &str,
    uepm_plugins_dir: &Path,
    token: Option<&str>,
) -> Result<String, UepmError>   // returns the resolved 40-char commit SHA
```

Steps:

1. **Parse** — strip the `git:` prefix; split on `#` to get `(url, ref_or_head)`. Default ref is `HEAD` when absent.

2. **Auth injection** — if `token` is `Some(t)` and the URL scheme is `https://` or `http://`, rewrite to `https://<token>@<host>/<path>`. SSH URLs pass through untouched; the system SSH agent handles private repos natively.

3. **Destination** — `dest = uepm_plugins_dir / plugin_dir_name(package_name)`. Any existing entry at `dest` (symlink, file, or directory) is removed before cloning, matching the behaviour of `symlink_local`.

4. **Clone**:
   - For a branch or tag ref: `git clone --depth 1 --branch <ref> <url> <dest>`
   - For a 40-character hex SHA: `git clone --depth 1 <url> <dest>` then `git -C <dest> fetch --depth 1 origin <sha>` + `git -C <dest> checkout FETCH_HEAD`
   - For `HEAD` (no ref): `git clone --depth 1 <url> <dest>`

5. **Resolve SHA** — `git -C <dest> rev-parse HEAD`, captured from stdout, trimmed to 40 characters.

6. **Return** the SHA.

`stderr` is inherited in all subprocess calls so git's progress output is visible to the user. `stdout` is captured only for `rev-parse`. A non-zero exit code returns `UepmError::GitError`. A missing `git` binary returns `UepmError::GitNotFound`.

---

## Changes by File

### `src/errors.rs`

Add three new variants:

```rust
#[error("git is not installed or not on PATH. Install git and try again.")]
GitNotFound,

#[error("git clone failed for {url}: {message}")]
GitError { url: String, message: String },

#[error("Invalid git: spec '{spec}': {message}")]
InvalidGitSpec { spec: String, message: String },
```

### `src/installer.rs`

Add `pub async fn clone_git(…)` as described above. Add two private helpers:

- `parse_git_spec(spec: &str) -> Result<(String, String), UepmError>` — splits `git:<url>#<ref>` into `(url, ref)`.
- `inject_token(url: &str, token: &str) -> String` — rewrites HTTPS URLs with the token; returns the URL unchanged for SSH.
- `replace_ref_fragment(spec: &str, sha: &str) -> String` — rewrites `git:https://…#<anything>` → `git:https://…#<sha>`, or appends `#<sha>` if there was no fragment.

### `src/resolver.rs`

Add a `git:` branch in `resolve_and_install`, between the `file:` branch and the registry path:

```rust
} else if range.starts_with("git:") {
    // On a plain install, use the locked SHA for reproducibility.
    // On uepm update (ctx.ignore_lock = true), re-clone from the original spec.
    let effective_spec = if !ctx.ignore_lock {
        if let Some(locked) = ctx.lock.plugins.get(package) {
            replace_ref_fragment(&locked.tarball, &locked.resolved)
        } else {
            range.to_string()
        }
    } else {
        range.to_string()
    };

    if ctx.verbose {
        crate::output::print_info(&format!("Installing {package} from {effective_spec}"));
    }
    let sha = clone_git(&effective_spec, package, ctx.uepm_plugins_dir, ctx.token).await?;
    (sha, range.to_string(), String::new())
}
```

`git:` ranges skip `check_conflict` — two `git:` deps for the same package that both resolve to the same SHA in a session are compatible. A `git:` dep and a semver dep for the same package name is always an error; the existing conflict message covers this case because the SHA won't satisfy any semver range.

Add `pub ignore_lock: bool` to `ResolveContext`, defaulting to `false`. `commands/update.rs` sets it to `true` when constructing the context.

### `src/commands/install.rs`

Extend `parse_package_spec` to handle the `=`-separated scoped-name form:

```rust
fn parse_package_spec(spec: &str) -> (String, Option<&str>) {
    // @scope/name=git:… form
    if let Some(pos) = spec.find('=') {
        return (spec[..pos].to_string(), Some(&spec[pos + 1..]));
    }
    // bare git:… — derive name from URL
    if spec.starts_with("git:") {
        let name = derive_name_from_git_spec(spec);
        return (name, Some(spec));
    }
    // existing @-split logic unchanged
    match spec.rfind('@').filter(|&pos| pos > 0) {
        Some(pos) => (spec[..pos].to_string(), Some(&spec[pos + 1..])),
        None => (spec.to_string(), None),
    }
}
```

`git:` ranges bypass `fetch_metadata_for_version`. The pinned range written to `Config/UEPM.ini` is the original `git:…` spec verbatim (no `^` prefix).

### `src/lockfile.rs`

No structural changes. The existing fields carry the needed semantics for `git:` entries:

| Field | `git:` value |
|---|---|
| `resolved` | Full 40-char commit SHA |
| `tarball` | Original `git:…` spec (used by `uepm update` to re-resolve) |
| `sha512` | `""` — same convention as `file:` |
| `dependencies` | Populated from the cloned plugin's `Config/UEPM.ini` as usual |

---

## Auth

`UEPM_TOKEN` is injected into HTTPS URLs only: `https://github.com/org/repo` → `https://<token>@github.com/org/repo`. SSH URLs (`git@github.com:…`, `ssh://…`) are passed to `git` unchanged; the system SSH agent and `~/.ssh/config` handle authentication natively.

Private HTTPS repos that don't use a simple bearer token (e.g. GitHub's `x-access-token:` pattern) work without any UEPM-specific handling because `UEPM_TOKEN` is also set in the subprocess environment, and users' git credential helpers can pick it up. This is not documented as a first-class feature in this phase.

---

## `uepm update` Behaviour

`uepm update` sets `ctx.ignore_lock = true`. The `git:` branch in `resolve_and_install` then uses the original ref from `Config/UEPM.ini` rather than the locked SHA, re-clones, and writes the new HEAD SHA to `uepm.lock`. If the SHA is unchanged, the lockfile entry is overwritten with the same value — a no-op in practice.

---

## Error Handling

| Scenario | Error |
|---|---|
| `git` not on `PATH` | `UepmError::GitNotFound` |
| `git clone` exits non-zero | `UepmError::GitError { url, message }` |
| Malformed `git:` spec | `UepmError::InvalidGitSpec { spec, message }` |
| Cloned directory missing `.uplugin` | `UepmError::Io` (existing) — no `.uplugin` means version defaults to `"0.0.0"`, consistent with `symlink_local` |

---

## Testing

Unit tests in `src/installer.rs`:

- `test_parse_git_spec_https` — URL + ref extraction for `git:https://…#main`
- `test_parse_git_spec_no_ref` — `HEAD` default when no `#` is present
- `test_parse_git_spec_ssh` — SSH URL passes through `parse_git_spec` unchanged
- `test_replace_ref_fragment_replaces` — `git:https://…#main` + SHA → `git:https://…#<sha>`
- `test_replace_ref_fragment_appends` — no existing fragment → appends `#<sha>`
- `test_inject_token_https` — token `abc` + `https://github.com/…` → `https://abc@github.com/…`
- `test_inject_token_ssh_noop` — SSH URL is unchanged when token is `Some(…)`

Integration tests in `tests/git_integration.rs`, using a `bare` git repo created with `git init --bare` and a test commit inside `tempfile::tempdir()` to avoid network calls:

- `test_clone_git_installs_to_uepm_plugins` — clone a local bare repo, assert `UEPMPlugins/<name>/` exists with a `.uplugin` file, assert the returned SHA is 40 hex chars
- `test_clone_git_pinned_sha_reproducible` — first install pins a SHA; second call with the same spec re-uses the locked SHA without re-cloning (directory mtime is unchanged)
- `test_clone_git_replaces_existing_dir` — existing `UEPMPlugins/<name>/` directory is replaced cleanly
- `test_resolve_and_install_git_writes_lockfile` — assert `uepm.lock` has `resolved` (40-char SHA), `tarball` (`git:…` spec), and `sha512` (`""`)
- `test_update_re_clones_from_ref` — install pinned to an old SHA, call `resolve_and_install` with `ignore_lock: true`, assert the new SHA is written to the lockfile
- `test_git_not_found_returns_error` — override `PATH` to exclude `git`, assert `UepmError::GitNotFound`

---

## Dependencies

No new crates. `git` is invoked as a subprocess via `tokio::process::Command`, which is already in the dependency tree. This avoids pulling in `git2` (libgit2 bindings), which adds a C dependency that complicates the cross-compilation matrix in the release CI.

---

## Out of Scope

- **Submodules** — not automatically initialized. Documented as a known limitation.
- **Shallow-clone depth configuration** — always `--depth 1`.
- **`git:` sources for transitive deps** — these work automatically because `resolve_transitive_deps` calls `resolve_and_install`, which handles `git:` ranges.
- **Lockfile schema version bump** — the `tarball` field stores a `git:` spec rather than a tarball URL, but the shape is unchanged. A rename to `source` is deferred to a future lockfile v2.
- **`uepm list` display for `git:` installs** — the version column will show the 8-char SHA prefix. Left for the `uepm list` redesign that accompanies `uepm outdated`.
