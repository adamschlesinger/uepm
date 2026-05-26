//! Tarball builder for `uepm publish`.
//!
//! Walks a plugin directory, collects files according to npm-compatible
//! inclusion rules, and returns a `.tgz` as raw bytes.  The caller supplies
//! the `package.json` content; it is injected as `package/package.json`
//! without ever being written to disk.

use crate::errors::UepmError;
use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use std::path::{Path, PathBuf};

// ── Public API ────────────────────────────────────────────────────────────────

/// Build a `.tgz` tarball from `plugin_dir`.
///
/// Every entry is prefixed `package/` (npm convention).
/// `package_json` is injected as `package/package.json`.
pub fn build_tarball(plugin_dir: &Path, package_json: &[u8]) -> Result<Vec<u8>, UepmError> {
    let files = collect_files(plugin_dir)?;

    let mut buf: Vec<u8> = Vec::new();
    {
        let gz = GzEncoder::new(&mut buf, Compression::best());
        let mut tar = tar::Builder::new(gz);
        tar.follow_symlinks(false);

        // Inject package.json first (npm convention)
        append_bytes(&mut tar, "package/package.json", package_json)?;

        // Append all collected files
        for rel in &files {
            let abs = plugin_dir.join(rel);
            let tar_path = format!("package/{}", rel.display());
            append_file(&mut tar, &abs, &tar_path)?;
        }

        let gz = tar.into_inner().map_err(UepmError::Io)?;
        gz.finish().map_err(UepmError::Io)?;
    }

    Ok(buf)
}

/// Return a sorted list of files that would be packed (relative paths).
/// Useful for dry-run output.
pub fn list_files(plugin_dir: &Path) -> Result<Vec<PathBuf>, UepmError> {
    collect_files(plugin_dir)
}

// ── File collection ───────────────────────────────────────────────────────────

/// Always included regardless of ignore rules.
fn is_always_included(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == "package.json"
        || lower.starts_with("readme")
        || lower.starts_with("license")
        || lower.starts_with("changelog")
        || lower.ends_with(".uplugin")
}

/// Always excluded directories (short-circuit directory descent).
fn is_excluded_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules" | ".git" | "UEPMPlugins" | "Binaries" | "Intermediate" | ".uepm_cache"
    )
}

/// Always excluded files / patterns.
fn is_excluded_file(name: &str) -> bool {
    // Note: Config/UEPM.ini is intentionally included — consumers need the
    // [Dependencies] section to resolve transitive dependencies after extraction.
    name.ends_with(".lock")
        || name.starts_with(".npmrc")
        || name.starts_with(".env")
}

/// Walk `dir` recursively and return paths relative to `dir`.
/// Applies inclusion/exclusion rules matching npm's default behaviour.
fn collect_files(root: &Path) -> Result<Vec<PathBuf>, UepmError> {
    let mut out: Vec<PathBuf> = Vec::new();
    walk(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), UepmError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Use symlink_metadata so we don't follow symlinks — the tar builder
        // has follow_symlinks(false), so the walk must match that behaviour.
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if metadata.is_dir() {
            if !is_excluded_dir(&name_str) {
                walk(root, &path, out)?;
            }
        } else if metadata.is_file() {
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            let filename = rel
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            if is_always_included(&filename)
                || (!is_excluded_file(&filename) && !name_str.starts_with('.'))
            {
                out.push(rel);
            }
        }
    }
    Ok(())
}

// ── Tar helpers ───────────────────────────────────────────────────────────────

fn append_bytes<W: Write>(
    tar: &mut tar::Builder<W>,
    path: &str,
    data: &[u8],
) -> Result<(), UepmError> {
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(0); // reproducible
    header.set_cksum();
    tar.append_data(&mut header, path, data)
        .map_err(UepmError::Io)
}

fn append_file<W: Write>(
    tar: &mut tar::Builder<W>,
    abs: &Path,
    tar_path: &str,
) -> Result<(), UepmError> {
    let data = std::fs::read(abs)?;
    append_bytes(tar, tar_path, &data)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::read::GzDecoder;
    use std::collections::HashSet;
    use tar::Archive;
    use tempfile::tempdir;

    fn create_tree(dir: &Path, files: &[&str]) {
        for f in files {
            let p = dir.join(f);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(p, format!("content of {f}")).unwrap();
        }
    }

    fn tarball_entries(bytes: &[u8]) -> HashSet<String> {
        let gz = GzDecoder::new(bytes);
        let mut archive = Archive::new(gz);
        archive
            .entries()
            .unwrap()
            .flatten()
            .map(|e| e.path().unwrap().to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn test_tarball_entries_prefixed_package() {
        let dir = tempdir().unwrap();
        create_tree(
            dir.path(),
            &["MyPlugin.uplugin", "Source/MyPlugin/foo.cpp"],
        );
        let pkg_json = br#"{"name":"@acme/my-plugin","version":"1.0.0"}"#;
        let tgz = build_tarball(dir.path(), pkg_json).unwrap();
        let entries = tarball_entries(&tgz);

        assert!(entries.contains("package/package.json"));
        assert!(entries.contains("package/MyPlugin.uplugin"));
        assert!(entries.contains("package/Source/MyPlugin/foo.cpp"));
    }

    #[test]
    fn test_package_json_injected_from_arg_not_disk() {
        let dir = tempdir().unwrap();
        create_tree(dir.path(), &["MyPlugin.uplugin"]);
        // Write a different package.json to disk — it should be ignored
        std::fs::write(dir.path().join("package.json"), b"{}").unwrap();

        let injected = br#"{"name":"@acme/my-plugin","version":"2.0.0"}"#;
        let tgz = build_tarball(dir.path(), injected).unwrap();

        let gz = GzDecoder::new(tgz.as_slice());
        let mut archive = Archive::new(gz);
        let mut found_content = String::new();
        for entry in archive.entries().unwrap().flatten() {
            let mut e = entry;
            if e.path().unwrap().to_string_lossy() == "package/package.json" {
                std::io::Read::read_to_string(&mut e, &mut found_content).unwrap();
                break;
            }
        }
        // Should contain the injected content, not the disk file
        assert!(found_content.contains("2.0.0"));
    }

    #[test]
    fn test_excluded_dirs_not_packed() {
        let dir = tempdir().unwrap();
        create_tree(
            dir.path(),
            &[
                "MyPlugin.uplugin",
                "node_modules/some-pkg/index.js",
                "Binaries/Win64/plugin.dll",
                "Intermediate/Build/stuff",
                "UEPMPlugins/@dep/foo.uplugin",
            ],
        );
        let tgz = build_tarball(dir.path(), b"{}").unwrap();
        let entries = tarball_entries(&tgz);

        assert!(!entries.iter().any(|e| e.contains("node_modules")));
        assert!(!entries.iter().any(|e| e.contains("Binaries")));
        assert!(!entries.iter().any(|e| e.contains("Intermediate")));
        assert!(!entries.iter().any(|e| e.contains("UEPMPlugins")));
    }

    #[test]
    fn test_excluded_files_not_packed() {
        let dir = tempdir().unwrap();
        create_tree(
            dir.path(),
            &[
                "MyPlugin.uplugin",
                "uepm.lock",
                ".npmrc",
                ".env",
            ],
        );
        let tgz = build_tarball(dir.path(), b"{}").unwrap();
        let entries = tarball_entries(&tgz);

        assert!(!entries.iter().any(|e| e.contains(".lock")));
        assert!(!entries.iter().any(|e| e.contains(".npmrc")));
        assert!(!entries.iter().any(|e| e.contains(".env")));
    }

    #[test]
    fn test_uepm_ini_included_for_transitive_dep_resolution() {
        // Config/UEPM.ini must be packed so consumers can read [Dependencies]
        // and install transitive dependencies after extraction.
        let dir = tempdir().unwrap();
        create_tree(
            dir.path(),
            &[
                "MyPlugin.uplugin",
                "Config/UEPM.ini",
            ],
        );
        let tgz = build_tarball(dir.path(), b"{}").unwrap();
        let entries = tarball_entries(&tgz);
        assert!(
            entries.iter().any(|e| e.contains("UEPM.ini")),
            "Config/UEPM.ini must be included in tarball for transitive dep resolution"
        );
    }

    #[test]
    fn test_always_included_files() {
        assert!(is_always_included("README.md"));
        assert!(is_always_included("readme"));
        assert!(is_always_included("LICENSE"));
        assert!(is_always_included("CHANGELOG.md"));
        assert!(is_always_included("MyPlugin.uplugin"));
        assert!(is_always_included("package.json"));
        assert!(!is_always_included("main.cpp"));
    }

    #[test]
    fn test_list_files_matches_build_tarball() {
        let dir = tempdir().unwrap();
        create_tree(
            dir.path(),
            &["MyPlugin.uplugin", "Source/foo.cpp", "README.md"],
        );
        let listed = list_files(dir.path()).unwrap();
        let tgz = build_tarball(dir.path(), b"{}").unwrap();
        let entries = tarball_entries(&tgz);

        // Every listed file should appear in the tarball (as package/<rel>)
        for rel in &listed {
            let tar_path = format!("package/{}", rel.display());
            assert!(entries.contains(&tar_path), "missing {tar_path}");
        }
    }
}
