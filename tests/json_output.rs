use assert_cmd::Command;
use std::collections::HashMap;
use tempfile::tempdir;
use uepm::lockfile::{write_lockfile, LockedPlugin, LockFile};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn locked_plugin(version: &str, tarball: &str) -> LockedPlugin {
    LockedPlugin {
        resolved: version.to_string(),
        tarball: tarball.to_string(),
        sha512: "sha512-abc".to_string(),
        dependencies: HashMap::new(),
    }
}

fn uepm() -> Command {
    Command::cargo_bin("uepm").unwrap()
}

// ── list --json ───────────────────────────────────────────────────────────────

#[test]
fn test_list_json_returns_array() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(
        dir.path().join("Config/UEPM.ini"),
        "[Settings]\nEngineVersion = \"5.3.0\"\n\n[Dependencies]\n\"@acme/cool-plugin\" = \">=5.0.0, <6.0.0\"\n",
    )
    .unwrap();

    let mut lock = LockFile::default();
    lock.plugins.insert(
        "@acme/cool-plugin".to_string(),
        locked_plugin("1.2.3", "https://example.com/tarball.tgz"),
    );
    write_lockfile(dir.path(), &lock).unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "list"])
        .output()
        .unwrap();

    assert!(output.status.success(), "exit code should be 0");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be valid JSON");
    assert!(json.is_array(), "list --json must emit an array");
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "@acme/cool-plugin");
    assert_eq!(arr[0]["version"], "1.2.3");
    assert_eq!(arr[0]["compatible"], true);
}

#[test]
fn test_list_json_empty_returns_empty_array() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join("Config/UEPM.ini"), "[Dependencies]\n").unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "list"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json, serde_json::json!([]));
}

// ── uninstall --json ──────────────────────────────────────────────────────────

#[test]
fn test_uninstall_json_returns_removed_field() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins")).unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins/cool-plugin")).unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(
        dir.path().join("Config/UEPM.ini"),
        "[Dependencies]\n\"@acme/cool-plugin\" = \"^1.0.0\"\n",
    )
    .unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "uninstall", "@acme/cool-plugin"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["removed"], "@acme/cool-plugin");
}

// ── update --json (empty manifest, no network) ────────────────────────────────

#[test]
fn test_update_json_no_plugins_returns_empty_array() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join("Config/UEPM.ini"), "[Dependencies]\n").unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins")).unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "update"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json, serde_json::json!([]));
}

// ── install --json ────────────────────────────────────────────────────────────

use base64::{engine::general_purpose, Engine};
use mockito::Server;
use sha2::{Digest, Sha512};

fn make_fake_tarball() -> Vec<u8> {
    use flate2::{write::GzEncoder, Compression};
    use tar::Builder;
    let enc = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = Builder::new(enc);
    let content = b"{\"FileVersion\": 3}";
    let mut header = tar::Header::new_gnu();
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "package/CoolPlugin.uplugin", content.as_ref())
        .unwrap();
    builder.into_inner().unwrap().finish().unwrap()
}

fn sha512_integrity(data: &[u8]) -> String {
    format!(
        "sha512-{}",
        general_purpose::STANDARD.encode(Sha512::digest(data))
    )
}

fn registry_meta(pkg: &str, version: &str, tarball_url: &str, integrity: &str) -> serde_json::Value {
    serde_json::json!({
        "name": pkg,
        "dist-tags": { "latest": version },
        "versions": {
            version: {
                "name": pkg,
                "version": version,
                "dist": { "tarball": tarball_url, "integrity": integrity }
            }
        }
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn test_install_json_single_package_fresh() {
    let mut server = Server::new_async().await;
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join("Config/UEPM.ini"), "[Dependencies]\n").unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins")).unwrap();

    let tarball = make_fake_tarball();
    let integrity = sha512_integrity(&tarball);
    let tarball_url = format!("{}/tarball.tgz", server.url());
    let meta = registry_meta("@acme/cool-plugin", "1.0.0", &tarball_url, &integrity);

    let _meta_mock = server
        .mock("GET", "/%40acme%2Fcool-plugin")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta.to_string())
        .create_async()
        .await;
    let _tarball_mock = server
        .mock("GET", "/tarball.tgz")
        .with_status(200)
        .with_body(tarball)
        .create_async()
        .await;

    let output = uepm()
        .current_dir(dir.path())
        .env("UEPM_REGISTRY", server.url())
        .args(["--json", "install", "@acme/cool-plugin"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.is_array(), "install --json must emit an array");
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["name"], "@acme/cool-plugin");
    assert_eq!(arr[0]["version"], "1.0.0");
    assert_eq!(arr[0]["fresh"], true, "new package must be fresh:true");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_install_json_multiple_packages_is_array() {
    let mut server = Server::new_async().await;
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join("Config/UEPM.ini"), "[Dependencies]\n").unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins")).unwrap();

    let tarball_a = make_fake_tarball();
    let integrity_a = sha512_integrity(&tarball_a);
    let tarball_b = make_fake_tarball();
    let integrity_b = sha512_integrity(&tarball_b);

    let url_a = format!("{}/tarball_a.tgz", server.url());
    let url_b = format!("{}/tarball_b.tgz", server.url());

    let _ma = server
        .mock("GET", "/%40acme%2Fplugin-a")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(registry_meta("@acme/plugin-a", "1.0.0", &url_a, &integrity_a).to_string())
        .create_async()
        .await;
    let _ta = server
        .mock("GET", "/tarball_a.tgz")
        .with_status(200)
        .with_body(tarball_a)
        .create_async()
        .await;
    let _mb = server
        .mock("GET", "/%40acme%2Fplugin-b")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(registry_meta("@acme/plugin-b", "2.0.0", &url_b, &integrity_b).to_string())
        .create_async()
        .await;
    let _tb = server
        .mock("GET", "/tarball_b.tgz")
        .with_status(200)
        .with_body(tarball_b)
        .create_async()
        .await;

    let output = uepm()
        .current_dir(dir.path())
        .env("UEPM_REGISTRY", server.url())
        .args(["--json", "install", "@acme/plugin-a", "@acme/plugin-b"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.is_array(), "must be array for multiple packages");
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_install_json_already_locked_fresh_false() {
    let mut server = Server::new_async().await;
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(
        dir.path().join("Config/UEPM.ini"),
        "[Dependencies]\n\"@acme/cool-plugin\" = \"^1.0.0\"\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins")).unwrap();
    std::fs::create_dir(dir.path().join("UEPMPlugins/cool-plugin")).unwrap();

    // Pre-populate uepm.lock so the plugin is already locked.
    // Must use the real sha512 of the fake tarball so download_and_extract can verify it.
    let tarball = make_fake_tarball();
    let integrity = sha512_integrity(&tarball);
    let tarball_url = format!("{}/tarball.tgz", server.url());
    let mut lock = LockFile::default();
    lock.plugins.insert(
        "@acme/cool-plugin".to_string(),
        LockedPlugin {
            resolved: "1.0.0".to_string(),
            tarball: tarball_url.clone(),
            sha512: integrity.clone(),
            dependencies: HashMap::new(),
        },
    );
    write_lockfile(dir.path(), &lock).unwrap();

    let meta = registry_meta("@acme/cool-plugin", "1.0.0", &tarball_url, &integrity);
    let _mm = server
        .mock("GET", "/%40acme%2Fcool-plugin")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(meta.to_string())
        .create_async()
        .await;
    let _tm = server
        .mock("GET", "/tarball.tgz")
        .with_status(200)
        .with_body(tarball)
        .create_async()
        .await;

    let output = uepm()
        .current_dir(dir.path())
        .env("UEPM_REGISTRY", server.url())
        .args(["--json", "install"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(
        arr[0]["fresh"], false,
        "already-locked package must be fresh:false"
    );
}

// ── Error shape ───────────────────────────────────────────────────────────────

#[test]
fn test_error_emits_json_on_stdout_with_nonzero_exit() {
    let dir = tempdir().unwrap();
    // No Config/UEPM.ini — list will fail with a manifest parse error.

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "list"])
        .output()
        .unwrap();

    assert!(!output.status.success(), "non-zero exit expected on error");
    assert!(output.stdout.len() > 0, "error must be on stdout, not stderr");
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("error output must be valid JSON");
    assert!(
        json["error"].is_string(),
        "error JSON must have an 'error' string field"
    );
    assert_eq!(
        json.as_object().unwrap().len(),
        1,
        "error JSON must have exactly one field"
    );
}

#[test]
fn test_json_without_yes_on_init_exits_with_error_json() {
    let dir = tempdir().unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "init"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["error"].as_str().unwrap().contains("--yes"));
}

// ── publish --json --dry-run ──────────────────────────────────────────────────

#[test]
fn test_publish_json_dry_run_shape() {
    let dir = tempdir().unwrap();
    std::fs::create_dir(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join("CoolPlugin.uplugin"), r#"{"FileVersion": 3}"#).unwrap();
    std::fs::write(
        dir.path().join("Config/UEPM.ini"),
        "[Plugin]\nName = \"@acme/cool-plugin\"\nVersion = \"1.0.0\"\nEngineRange = \">=5.3.0, <6.0.0\"\nMain = \"CoolPlugin.uplugin\"\nAuthor = \"ACME\"\nLicense = \"MIT\"\nDescription = \"A plugin\"\n\n[Dependencies]\n",
    )
    .unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "publish", "--yes", "--dry-run"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["name"], "@acme/cool-plugin");
    assert_eq!(json["version"], "1.0.0");
    assert!(json["registry"].as_str().is_some_and(|r| !r.is_empty()));
    assert_eq!(json["dry_run"], true);
}

// ── init --json --yes (plugin context) ───────────────────────────────────────

#[test]
fn test_init_plugin_json_shape() {
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("CoolPlugin.uplugin"),
        r#"{"FriendlyName": "Cool Plugin", "VersionName": "2.0.0", "CreatedBy": "ACME"}"#,
    )
    .unwrap();

    let output = uepm()
        .current_dir(dir.path())
        .args(["--json", "init", "--yes"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["type"], "plugin");
    assert!(json["name"].as_str().is_some_and(|n| n.contains("acme")));
    assert_eq!(json["version"], "2.0.0");
    assert!(json["engine_range"].as_str().is_some_and(|r| !r.is_empty()));
    assert!(!json.as_object().unwrap().contains_key("vcs"), "vcs field must not be present");
}
