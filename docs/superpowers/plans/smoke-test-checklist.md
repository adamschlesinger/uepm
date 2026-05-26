# UEPM Smoke Test Checklist

Run this before tagging a release. All steps use the **local release binary**
so no GitHub release is needed first.

```sh
cargo build --release
export PATH="$(pwd)/target/release:$PATH"
uepm --version
```

---

## 1 — Project consumer flow

### 1.1 Init
```sh
cd /path/to/a/.uproject/dir
uepm init
```
- [ ] `Config/UEPM.ini` created with `EngineVersion` and `CommitPlugins`
- [ ] `UEPMPlugins/` directory created
- [ ] `.uproject` now contains `"AdditionalPluginDirectories": ["UEPMPlugins"]`
- [ ] `.gitignore` updated (if git repo and CommitPlugins=false)

### 1.2 Install
```sh
uepm install @acme/cool-plugin        # replace with a real package on your registry
```
- [ ] Plugin directory appears under `UEPMPlugins/`
- [ ] `Config/UEPM.ini` has the package entry
- [ ] `uepm.lock` written with resolved version + sha512

### 1.3 List
```sh
uepm list
```
- [ ] Shows installed plugin with resolved version
- [ ] Shows engine compatibility status (✓ or ✗)

### 1.4 Update
```sh
uepm update
```
- [ ] Re-resolves and rewrites `uepm.lock` (or reports already up to date)

### 1.5 Uninstall
```sh
uepm uninstall @acme/cool-plugin
```
- [ ] `UEPMPlugins/<name>/` removed
- [ ] Entry removed from `Config/UEPM.ini`

---

## 2 — Plugin author flow

### 2.1 Init (plugin context)
```sh
cd /path/to/a/.uplugin/dir
uepm init
```
- [ ] Detected `.uplugin` — did NOT ask for `CommitPlugins`
- [ ] Prompted for package name, version, description, author, license, engine range, main
- [ ] Defaults pre-filled from `.uplugin` fields (FriendlyName, VersionName, etc.)
- [ ] Engine range default derived from installed UE (or `>=5.x.0, <6.0.0` if none found)
- [ ] `Config/UEPM.ini` written with `[Package]` section
- [ ] Re-running prompts "already exists — overwrite?" confirm

### 2.2 Publish dry-run
```sh
export UEPM_TOKEN=any-value
uepm publish --dry-run
```
- [ ] Prints publish summary (name, version, engine range, registry, tag, access)
- [ ] Lists files that would be packed (`package/` prefixed)
- [ ] Shows tarball size in bytes
- [ ] Exits with "Dry run — skipping upload" — **no HTTP request made**
- [ ] `package.json` was **not** written to disk

### 2.3 Publish missing token
```sh
unset UEPM_TOKEN
uepm publish --yes
```
- [ ] Exits with `UEPM_TOKEN is not set` error

### 2.4 Publish missing [Package]
```sh
cd /path/to/project/with/no/[Package]
uepm publish --yes
```
- [ ] Exits with `No [Package] section found` error

### 2.5 Publish to a local registry (optional — requires Verdaccio)
```sh
# start: npx verdaccio &
export UEPM_REGISTRY=http://localhost:4873
export UEPM_TOKEN=$(npm token create --registry http://localhost:4873 | ...)
uepm publish --yes
```
- [ ] PUT request reaches registry
- [ ] Registry confirms package published
- [ ] `uepm install @scope/name` from a separate project dir installs the just-published package

---

## 3 — Install script

Push a pre-release tag (e.g. `v2.0.0-rc.1`) and wait for CI to build binaries.
Then test the override path:

```sh
# macOS/Linux
UEPM_VERSION=v2.0.0-rc.1 sh install.sh
~/.uepm/bin/uepm --version

# PowerShell (Windows)
$env:UEPM_VERSION="v2.0.0-rc.1"; irm https://.../install.ps1 | iex
uepm --version
```
- [ ] Binary downloaded to `~/.uepm/bin/uepm`
- [ ] PATH line added to `.zshrc` / `.bashrc`
- [ ] `uepm --version` prints the expected version
- [ ] Running install script a second time is idempotent (no duplicate PATH entries)

---

## 4 — Edge cases

```sh
# No .uproject or .uplugin in cwd
uepm init
```
- [ ] Sensible error (not a panic)

```sh
# Bad package name
uepm install not-scoped
```
- [ ] Error explaining scope requirement

```sh
# uepm publish with invalid version in [Package]
# Set Version = "not-semver" in Config/UEPM.ini
uepm publish --dry-run
```
- [ ] `InvalidPackageField: version` error

```sh
# uepm publish with missing .uplugin file
# Set Main = "Ghost.uplugin" but don't create the file
uepm publish --dry-run
```
- [ ] `InvalidPackageField: main` error
