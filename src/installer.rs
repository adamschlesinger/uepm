use crate::errors::UepmError;
use base64::{engine::general_purpose, Engine};
use bytes::Bytes;
use sha2::{Digest, Sha512};
use std::path::Path;

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

    let plugin_dir_name = package_name.split('/').last().unwrap_or(package_name);
    let dest = uepm_plugins_dir.join(plugin_dir_name);

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

/// Create a symlink at `uepm_plugins_dir/<plugin_dir_name>` pointing to `src`.
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

    let dir_name = package_name.split('/').last().unwrap_or(package_name);
    let dest = uepm_plugins_dir.join(dir_name);

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
    let dir = std::fs::read_dir(plugin_dir).ok()?;
    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("uplugin") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                return json["VersionName"].as_str().map(String::from);
            }
        }
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
        use sha2::Digest;
        let hash = Sha512::digest(data);
        general_purpose::STANDARD.encode(hash)
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
