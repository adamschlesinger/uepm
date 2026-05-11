use base64::{engine::general_purpose, Engine};
use mockito::Server;
use serde_json::json;
use sha2::{Digest, Sha512};
use tempfile::tempdir;

fn make_fake_tarball() -> Vec<u8> {
    use flate2::{write::GzEncoder, Compression};
    use tar::Builder;
    let buf = Vec::new();
    let enc = GzEncoder::new(buf, Compression::default());
    let mut builder = Builder::new(enc);
    let content = b"{\"FileVersion\": 3, \"Version\": 1}";
    let mut header = tar::Header::new_gnu();
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "package/CoolPlugin.uplugin", content.as_ref())
        .unwrap();
    let enc = builder.into_inner().unwrap();
    enc.finish().unwrap()
}

fn sha512_integrity(data: &[u8]) -> String {
    format!(
        "sha512-{}",
        general_purpose::STANDARD.encode(Sha512::digest(data))
    )
}

#[tokio::test]
async fn test_install_single_plugin() {
    let dir = tempdir().unwrap();
    let uepm_dir = dir.path().join("UEPMPlugins");
    std::fs::create_dir(&uepm_dir).unwrap();

    std::fs::write(dir.path().join("uepm.ini"), "[plugins]\n").unwrap();

    let tarball = make_fake_tarball();
    let integrity = sha512_integrity(&tarball);
    let mut server = Server::new_async().await;

    let tarball_url = format!("{}/tarball.tgz", server.url());
    let meta = json!({
        "name": "@acme/cool-plugin",
        "dist-tags": { "latest": "1.0.0" },
        "versions": {
            "1.0.0": {
                "name": "@acme/cool-plugin",
                "version": "1.0.0",
                "dist": {
                    "tarball": tarball_url,
                    "integrity": integrity,
                }
            }
        }
    });

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

    std::env::set_var("UEPM_REGISTRY", server.url());

    uepm::commands::install::run_install(&["@acme/cool-plugin".to_string()], dir.path())
        .await
        .unwrap();

    std::env::remove_var("UEPM_REGISTRY");

    assert!(uepm_dir
        .join("cool-plugin")
        .join("CoolPlugin.uplugin")
        .exists());

    let manifest = uepm::manifest::read_manifest(dir.path()).unwrap();
    assert!(manifest.plugins.contains_key("@acme/cool-plugin"));

    let lock = uepm::lockfile::read_lockfile(dir.path()).unwrap().unwrap();
    assert!(lock.plugins.contains_key("@acme/cool-plugin"));
}
