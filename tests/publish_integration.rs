use mockito::Server;
use tempfile::tempdir;
use uepm::context::UEPMContext;
use uepm::manifest::{write_package_metadata, PackageMetadata};

fn write_uplugin(dir: &std::path::Path, name: &str) {
    std::fs::write(
        dir.join(format!("{name}.uplugin")),
        serde_json::to_vec(&serde_json::json!({"FileVersion": 3})).unwrap(),
    )
    .unwrap();
}

fn setup_plugin_dir(dir: &std::path::Path, meta: &PackageMetadata) {
    // Write the .uplugin file that [Plugin].main points to
    let uplugin_name = meta.main.trim_end_matches(".uplugin");
    write_uplugin(dir, uplugin_name);
    // Write Config/UEPM.ini with [Plugin] section
    std::fs::create_dir_all(dir.join("Config")).unwrap();
    std::fs::write(dir.join("Config/UEPM.ini"), "").unwrap();
    write_package_metadata(dir, meta).unwrap();
}

fn valid_meta() -> PackageMetadata {
    PackageMetadata {
        name: "@acme/my-plugin".to_string(),
        version: "1.2.0".to_string(),
        description: "Does stuff".to_string(),
        author: "ACME".to_string(),
        license: "MIT".to_string(),
        engine_range: ">=5.3.0, <6.0.0".to_string(),
        main: "MyPlugin.uplugin".to_string(),
    }
}

// ── dry-run ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_publish_dry_run_makes_no_http_request() {
    let dir = tempdir().unwrap();
    setup_plugin_dir(dir.path(), &valid_meta());

    // Server with no mocks — any request would fail the assertion
    let mut server = Server::new_async().await;
    let guard = server
        .mock("PUT", mockito::Matcher::Any)
        .expect(0)
        .create_async()
        .await;

    // No token needed for dry-run (check happens after tarball build)
    let ctx = UEPMContext::for_test(dir.path().to_path_buf(), &server.url(), Some("tok".into()));

    let result = uepm::commands::publish::run(&ctx, "latest", /*dry_run=*/ true, /*yes=*/ true, "public").await;

    assert!(result.is_ok(), "dry-run should succeed: {result:?}");
    guard.assert_async().await; // zero PUT calls
}

// ── successful publish ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_publish_success_puts_correct_headers() {
    let dir = tempdir().unwrap();
    setup_plugin_dir(dir.path(), &valid_meta());

    let mut server = Server::new_async().await;
    let mock = server
        .mock("PUT", "/%40acme%2Fmy-plugin")
        .match_header("authorization", "Bearer test-token")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_body("{}")
        .create_async()
        .await;

    let ctx = UEPMContext::for_test(
        dir.path().to_path_buf(),
        &server.url(),
        Some("test-token".into()),
    );

    let result = uepm::commands::publish::run(&ctx, "latest", false, /*yes=*/ true, "public").await;
    assert!(result.is_ok(), "publish should succeed: {result:?}");
    mock.assert_async().await;
}

#[tokio::test]
async fn test_publish_success_body_contains_required_fields() {
    let dir = tempdir().unwrap();
    setup_plugin_dir(dir.path(), &valid_meta());

    let mut server = Server::new_async().await;
    let mock = server
        .mock("PUT", "/%40acme%2Fmy-plugin")
        .match_body(mockito::Matcher::PartialJsonString(
            r#"{"_id":"@acme/my-plugin","name":"@acme/my-plugin"}"#.to_string(),
        ))
        .with_status(200)
        .with_body("{}")
        .create_async()
        .await;

    let ctx = UEPMContext::for_test(dir.path().to_path_buf(), &server.url(), Some("tok".into()));

    uepm::commands::publish::run(&ctx, "latest", false, true, "public")
        .await
        .unwrap();

    mock.assert_async().await;
}

// ── error handling ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_publish_401_without_otp_surfaces_registry_error() {
    let dir = tempdir().unwrap();
    setup_plugin_dir(dir.path(), &valid_meta());

    let mut server = Server::new_async().await;
    let _mock = server
        .mock("PUT", "/%40acme%2Fmy-plugin")
        .with_status(401)
        .with_header("content-type", "application/json")
        .with_body(r#"{"error":"You must be logged in to publish packages."}"#)
        .create_async()
        .await;

    let ctx = UEPMContext::for_test(dir.path().to_path_buf(), &server.url(), Some("bad-tok".into()));

    let result = uepm::commands::publish::run(&ctx, "latest", false, true, "public").await;

    match result {
        Err(uepm::errors::UepmError::PublishFailed(msg)) => {
            assert!(
                msg.contains("logged in") || msg.contains("401"),
                "expected registry error message, got: {msg}"
            );
        }
        other => panic!("expected PublishFailed, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_publish_registry_500_returns_publish_failed() {
    let dir = tempdir().unwrap();
    setup_plugin_dir(dir.path(), &valid_meta());

    let mut server = Server::new_async().await;
    let _mock = server
        .mock("PUT", "/%40acme%2Fmy-plugin")
        .with_status(500)
        .with_body(r#"{"error":"internal server error"}"#)
        .create_async()
        .await;

    let ctx = UEPMContext::for_test(dir.path().to_path_buf(), &server.url(), Some("tok".into()));

    let result = uepm::commands::publish::run(&ctx, "latest", false, true, "public").await;
    assert!(
        matches!(result, Err(uepm::errors::UepmError::PublishFailed(_))),
        "expected PublishFailed, got: {result:?}"
    );
}

// ── missing token ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_publish_without_token_returns_token_required() {
    let dir = tempdir().unwrap();
    setup_plugin_dir(dir.path(), &valid_meta());

    // token: None — no env var read
    let ctx = UEPMContext::for_test(dir.path().to_path_buf(), "https://registry.npmjs.org", None);

    let result = uepm::commands::publish::run(&ctx, "latest", false, true, "public").await;
    assert!(
        matches!(result, Err(uepm::errors::UepmError::TokenRequired)),
        "expected TokenRequired, got: {result:?}"
    );
}

// ── missing [Plugin] ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_publish_without_package_section_returns_error() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("Config")).unwrap();
    std::fs::write(dir.path().join("Config/UEPM.ini"), "[Dependencies]\n").unwrap();

    let ctx = UEPMContext::for_test(dir.path().to_path_buf(), "https://registry.npmjs.org", Some("tok".into()));

    let result = uepm::commands::publish::run(&ctx, "latest", false, true, "public").await;
    assert!(
        matches!(result, Err(uepm::errors::UepmError::NoPackageMetadata)),
        "expected NoPackageMetadata, got: {result:?}"
    );
}
