use crate::errors::UepmError;
use crate::resolver::plugin_dir_name;
use base64::{engine::general_purpose, Engine};
use bytes::Bytes;
use sha2::{Digest, Sha512};
use std::path::Path;

/// Download a tarball from `tarball_url`, verify its `sha512` integrity string,
/// and extract it into `uepm_plugins_dir/<plugin_name>/`.
/// The npm `package/` prefix inside the tarball is stripped on extraction.
pub async fn download_and_extract(
    tarball_url: &str,
    integrity: &str,
    package_name: &str,
    uepm_plugins_dir: &Path,
    token: Option<&str>,
) -> Result<(), UepmError> {
    tracing::debug!("Downloading {tarball_url}");

    let client = reqwest::Client::new();
    let mut req = client.get(tarball_url);
    if let Some(tok) = token {
        req = req.bearer_auth(tok);
    }
    let resp = req.send().await?.error_for_status()?;
    let data: Bytes = resp.bytes().await?;

    verify_integrity(&data, integrity, package_name)?;

    let dest = uepm_plugins_dir.join(plugin_dir_name(package_name));
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    std::fs::create_dir_all(&dest)?;

    extract_tarball(&data, &dest)?;

    Ok(())
}

fn verify_integrity(data: &[u8], integrity: &str, package: &str) -> Result<(), UepmError> {
    let b64 = integrity
        .strip_prefix("sha512-")
        .ok_or_else(|| UepmError::ChecksumMismatch {
            package: package.to_string(),
            expected: integrity.to_string(),
            actual: "(unparseable integrity string)".to_string(),
        })?;

    let expected = general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| UepmError::ChecksumMismatch {
            package: package.to_string(),
            expected: integrity.to_string(),
            actual: "(base64 decode failed)".to_string(),
        })?;

    let actual = Sha512::digest(data);

    if actual.as_slice() != expected.as_slice() {
        return Err(UepmError::ChecksumMismatch {
            package: package.to_string(),
            expected: general_purpose::STANDARD.encode(&expected),
            actual: general_purpose::STANDARD.encode(actual),
        });
    }

    Ok(())
}

fn extract_tarball(data: &[u8], dest: &Path) -> Result<(), UepmError> {
    let cursor = std::io::Cursor::new(data);
    let decoder = flate2::read::GzDecoder::new(cursor);
    let mut archive = tar::Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?.to_path_buf();

        // Reject absolute paths and any path that contains a `..` component —
        // either would allow a malicious tarball to write outside `dest`.
        if entry_path.is_absolute()
            || entry_path
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(UepmError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!(
                    "path traversal detected in tarball: {}",
                    entry_path.display()
                ),
            )));
        }

        let stripped = entry_path
            .strip_prefix("package")
            .unwrap_or(&entry_path);

        if stripped.as_os_str().is_empty() {
            continue;
        }

        let target = dest.join(stripped);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        entry.unpack(&target)?;
    }

    Ok(())
}

/// Create a symlink at `uepm_plugins_dir/<plugin_dir_name>` pointing directly at `src`,
/// so edits to the local plugin directory are immediately visible without reinstalling.
/// Returns the version read from the first `.uplugin` file in `src`, or `"0.0.0"`.
/// Any existing entry at the destination (symlink, file, or directory) is replaced.
pub fn symlink_local(
    src: &Path,
    package_name: &str,
    uepm_plugins_dir: &Path,
) -> Result<String, UepmError> {
    if !src.exists() {
        return Err(UepmError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("local plugin path not found: {}", src.display()),
        )));
    }

    let abs_src = src.canonicalize()?;
    let version = read_uplugin_version(&abs_src).unwrap_or_else(|| "0.0.0".to_string());

    let dest = uepm_plugins_dir.join(plugin_dir_name(package_name));

    if dest.symlink_metadata().is_ok() {
        if dest.is_symlink() || dest.is_file() {
            std::fs::remove_file(&dest)?;
        } else {
            std::fs::remove_dir_all(&dest)?;
        }
    }

    #[cfg(unix)]
    std::os::unix::fs::symlink(&abs_src, &dest)?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&abs_src, &dest)?;

    Ok(version)
}

fn read_uplugin_version(plugin_dir: &Path) -> Option<String> {
    for entry in std::fs::read_dir(plugin_dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("uplugin") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else { continue };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else { continue };
        return json["VersionName"].as_str().map(String::from);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use tempfile::tempdir;

    fn make_fake_tarball() -> Vec<u8> {
        use flate2::{write::GzEncoder, Compression};
        use tar::Builder;

        let buf = Vec::new();
        let enc = GzEncoder::new(buf, Compression::default());
        let mut builder = Builder::new(enc);

        let content = b"{\"FileVersion\": 3}";
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        builder
            .append_data(&mut header, "package/TestPlugin.uplugin", content.as_ref())
            .unwrap();

        let enc = builder.into_inner().unwrap();
        enc.finish().unwrap()
    }

    fn sha512_b64(data: &[u8]) -> String {
        general_purpose::STANDARD.encode(Sha512::digest(data))
    }

    #[tokio::test]
    async fn test_extract_installs_to_uepm_plugins() {
        let dir = tempdir().unwrap();
        let uepm_dir = dir.path().join("UEPMPlugins");
        std::fs::create_dir(&uepm_dir).unwrap();

        let tarball = make_fake_tarball();
        let integrity = format!("sha512-{}", sha512_b64(&tarball));

        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/tarball.tgz")
            .with_status(200)
            .with_header("content-type", "application/octet-stream")
            .with_body(tarball)
            .create_async()
            .await;

        let tarball_url = format!("{}/tarball.tgz", server.url());
        download_and_extract(
            &tarball_url,
            &integrity,
            "@test/test-plugin",
            &uepm_dir,
            None,
        )
        .await
        .unwrap();

        assert!(uepm_dir
            .join("test-plugin")
            .join("TestPlugin.uplugin")
            .exists());
    }

    #[test]
    fn test_symlink_local_creates_symlink_and_reads_version() {
        let src = tempdir().unwrap();
        std::fs::write(
            src.path().join("TestPlugin.uplugin"),
            r#"{"FileVersion": 3, "VersionName": "1.2.3"}"#,
        )
        .unwrap();
        std::fs::create_dir(src.path().join("Source")).unwrap();
        std::fs::write(src.path().join("Source/MyFile.cpp"), "// code").unwrap();

        let dest_root = tempdir().unwrap();
        let uepm_dir = dest_root.path().join("UEPMPlugins");
        std::fs::create_dir(&uepm_dir).unwrap();

        let version = symlink_local(src.path(), "@acme/test-plugin", &uepm_dir).unwrap();
        let dest = uepm_dir.join("test-plugin");

        assert_eq!(version, "1.2.3");
        assert!(dest.is_symlink());
        // Contents accessible through symlink
        assert!(dest.join("TestPlugin.uplugin").exists());
        assert!(dest.join("Source/MyFile.cpp").exists());
    }

    #[test]
    fn test_symlink_local_replaces_existing_symlink() {
        let src = tempdir().unwrap();
        std::fs::write(
            src.path().join("Plugin.uplugin"),
            r#"{"FileVersion": 3, "VersionName": "2.0.0"}"#,
        )
        .unwrap();

        let dest_root = tempdir().unwrap();
        let uepm_dir = dest_root.path().join("UEPMPlugins");
        std::fs::create_dir(&uepm_dir).unwrap();

        symlink_local(src.path(), "@acme/plug", &uepm_dir).unwrap();
        // Second call should not error
        let version = symlink_local(src.path(), "@acme/plug", &uepm_dir).unwrap();
        assert_eq!(version, "2.0.0");
        assert!(uepm_dir.join("plug").is_symlink());
    }

    #[test]
    fn test_symlink_local_missing_path_errors() {
        let dest_root = tempdir().unwrap();
        let uepm_dir = dest_root.path().join("UEPMPlugins");
        std::fs::create_dir(&uepm_dir).unwrap();

        let result = symlink_local(Path::new("/nonexistent/path"), "@acme/test-plugin", &uepm_dir);
        assert!(result.is_err());
    }

    /// Build a minimal .tar.gz with one entry whose stored path is `entry_path`.
    /// We write raw POSIX tar headers so we can embed paths the builder would reject.
    fn make_raw_tarball_with_path(entry_path: &str, content: &[u8]) -> Vec<u8> {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write;

        // A POSIX ustar header is 512 bytes
        let mut header = [0u8; 512];
        // Name field: bytes 0..100
        let name_bytes = entry_path.as_bytes();
        let name_len = name_bytes.len().min(99);
        header[..name_len].copy_from_slice(&name_bytes[..name_len]);
        // File mode: bytes 100..108 ("0000644\0")
        header[100..108].copy_from_slice(b"0000644\0");
        // UID / GID
        header[108..116].copy_from_slice(b"0000000\0");
        header[116..124].copy_from_slice(b"0000000\0");
        // File size in octal, bytes 124..136
        let size_str = format!("{:011o}\0", content.len());
        header[124..136].copy_from_slice(size_str.as_bytes());
        // mtime
        header[136..148].copy_from_slice(b"00000000000\0");
        // typeflag '0' = regular file
        header[156] = b'0';
        // magic "ustar  \0"
        header[257..265].copy_from_slice(b"ustar  \0");
        // Compute checksum (bytes 148..156 = spaces during calculation)
        header[148..156].copy_from_slice(b"        ");
        let cksum: u32 = header.iter().map(|&b| b as u32).sum();
        let cksum_str = format!("{:06o}\0 ", cksum);
        header[148..156].copy_from_slice(cksum_str.as_bytes());

        // Pad content to 512-byte block
        let padded_len = ((content.len() + 511) / 512) * 512;
        let mut tar_bytes = Vec::new();
        tar_bytes.extend_from_slice(&header);
        tar_bytes.extend_from_slice(content);
        tar_bytes.extend(std::iter::repeat(0u8).take(padded_len - content.len()));
        // Two 512-byte zero blocks = end-of-archive
        tar_bytes.extend([0u8; 1024]);

        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        gz.write_all(&tar_bytes).unwrap();
        gz.finish().unwrap()
    }

    #[test]
    fn test_extract_rejects_dotdot_path_traversal() {
        let dest = tempdir().unwrap();
        let tarball = make_raw_tarball_with_path("package/../../evil.sh", b"evil");
        let result = extract_tarball(&tarball, dest.path());
        assert!(
            result.is_err(),
            "tarball with .. path component must be rejected"
        );
    }

    #[tokio::test]
    async fn test_checksum_mismatch_aborts() {
        let dir = tempdir().unwrap();
        let uepm_dir = dir.path().join("UEPMPlugins");
        std::fs::create_dir(&uepm_dir).unwrap();

        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/tarball.tgz")
            .with_status(200)
            .with_body(b"bad data".as_ref())
            .create_async()
            .await;

        let result = download_and_extract(
            &format!("{}/tarball.tgz", server.url()),
            "sha512-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "@test/plugin",
            &uepm_dir,
            None,
        )
        .await;

        assert!(matches!(result, Err(UepmError::ChecksumMismatch { .. })));
    }
}
