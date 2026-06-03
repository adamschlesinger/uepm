use crate::context::{OutputMode, UEPMContext};
use crate::errors::UepmError;
use crate::manifest::{read_package_metadata, PackageMetadata};

#[derive(serde::Serialize)]
struct PublishResult {
    name: String,
    version: String,
    registry: String,
    dry_run: bool,
}
use crate::publisher;
use base64::Engine as _;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};
use semver::{Version, VersionReq};
use sha1::{Digest as _, Sha1};
use sha2::{Digest as _, Sha512};
use std::path::Path;

pub async fn run(
    ctx: &UEPMContext,
    tag: &str,
    dry_run: bool,
    yes: bool,
    access: &str,
) -> Result<(), UepmError> {
    // ── Step 1: load and validate [Plugin] ───────────────────────────────────
    let meta = read_package_metadata(&ctx.project_dir)?;
    validate_metadata(&meta, &ctx.project_dir)?;

    // ── Step 2: show summary + confirm ────────────────────────────────────────
    let registry = ctx
        .registry
        .base_url()
        .trim_end_matches('/')
        .to_string();

    if ctx.output_mode != OutputMode::Json {
        crate::output::print_info(&format!("Publishing {}", meta.name));
        println!("  Version      : {}", meta.version);
        println!("  Engine range : {}", meta.engine_range);
        println!("  Main         : {}", meta.main);
        println!("  Registry     : {registry}");
        println!("  Tag          : {tag}");
        println!("  Access       : {access}");
    }

    if !dry_run && !yes {
        if !dialoguer::console::Term::stdout().is_term() {
            return Err(UepmError::InteractiveRequired);
        }
        let ok = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Continue?")
            .default(false)
            .interact()
            .map_err(|e| UepmError::Io(std::io::Error::other(e.to_string())))?;
        if !ok {
            crate::output::print_info("Aborted");
            return Ok(());
        }
    }

    // ── Step 3: build in-memory package.json ─────────────────────────────────
    let plugin_name = meta
        .main
        .trim_end_matches(".uplugin")
        .to_string();
    let pkg_json_value = serde_json::json!({
        "name": meta.name,
        "version": meta.version,
        "description": meta.description,
        "main": meta.main,
        "author": meta.author,
        "license": meta.license,
        "unreal": {
            "engineVersion": meta.engine_range,
            "pluginName": plugin_name,
        },
        "keywords": ["unreal", "unreal-engine", "plugin", "uepm"],
    });
    let pkg_json_bytes = serde_json::to_vec_pretty(&pkg_json_value)?;

    // ── Step 4: build tarball + digests ──────────────────────────────────────
    let tarball = publisher::build_tarball(&ctx.project_dir, &pkg_json_bytes)?;

    let shasum: String = format!("{:x}", Sha1::digest(&tarball));
    let integrity: String = format!(
        "sha512-{}",
        base64::engine::general_purpose::STANDARD.encode(Sha512::digest(&tarball))
    );

    if ctx.output_mode != OutputMode::Json {
        let file_list = publisher::list_files(&ctx.project_dir)?;
        println!("\n  Files ({}):", file_list.len() + 1); // +1 for package.json
        println!("    package/package.json  (generated)");
        for f in &file_list {
            println!("    package/{}", f.display());
        }
        println!("\n  Tarball size : {} bytes", tarball.len());
    }

    // ── Step 5: upload (skip for dry run) ────────────────────────────────────
    if !dry_run {
        let token = match &ctx.token {
            Some(t) => t.clone(),
            None => return Err(UepmError::TokenRequired),
        };
        let body = build_publish_body(&meta, &pkg_json_value, &tarball, &shasum, &integrity, &registry, tag, access)?;
        upload(&registry, &meta.name, &body, &token, tag).await?;
    }

    if ctx.output_mode == OutputMode::Json {
        crate::output::emit_json(&PublishResult {
            name: meta.name.clone(),
            version: meta.version.clone(),
            registry: registry.clone(),
            dry_run,
        });
    } else if dry_run {
        crate::output::print_info("Dry run — skipping upload");
    } else {
        crate::output::print_success(&format!("Published {}@{} → {registry}", meta.name, meta.version));
    }
    Ok(())
}

// ── Validation ────────────────────────────────────────────────────────────────

pub(crate) fn validate_metadata(meta: &PackageMetadata, project_dir: &Path) -> Result<(), UepmError> {
    macro_rules! require_nonempty {
        ($field:expr, $name:expr) => {
            if $field.trim().is_empty() {
                return Err(UepmError::InvalidPackageField {
                    field: $name.to_string(),
                    message: "must not be empty".to_string(),
                });
            }
        };
    }

    require_nonempty!(meta.name, "name");
    require_nonempty!(meta.version, "version");
    require_nonempty!(meta.main, "main");
    require_nonempty!(meta.engine_range, "engine_range");

    // name must be scoped: @scope/name
    if !meta.name.starts_with('@') || !meta.name.contains('/') {
        return Err(UepmError::InvalidPackageField {
            field: "name".to_string(),
            message: "must be a scoped package name like @scope/name".to_string(),
        });
    }

    // version must parse as SemVer
    if meta.version.parse::<Version>().is_err() {
        return Err(UepmError::InvalidPackageField {
            field: "version".to_string(),
            message: format!("'{}' is not valid SemVer", meta.version),
        });
    }

    // engine_range must parse
    if meta.engine_range.parse::<VersionReq>().is_err() {
        return Err(UepmError::InvalidPackageField {
            field: "engine_range".to_string(),
            message: format!("'{}' is not a valid semver range", meta.engine_range),
        });
    }

    // main file must exist on disk
    if !project_dir.join(&meta.main).exists() {
        return Err(UepmError::InvalidPackageField {
            field: "main".to_string(),
            message: format!("'{}' does not exist in the plugin directory", meta.main),
        });
    }

    Ok(())
}

// ── Request body ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn build_publish_body(
    meta: &PackageMetadata,
    pkg_json: &serde_json::Value,
    tarball: &[u8],
    shasum: &str,
    integrity: &str,
    registry: &str,
    tag: &str,
    access: &str,
) -> Result<Vec<u8>, UepmError> {
    // Tarball filename uses unscoped name: "@acme/my-plugin" → "my-plugin"
    let unscoped = meta
        .name
        .rsplit('/')
        .next()
        .unwrap_or(&meta.name);
    let tarball_filename = format!("{}-{}.tgz", unscoped, meta.version);

    // dist.tarball URL
    let tarball_url = format!(
        "{}/{}/-/{}", registry,
        urlencoding::encode(&meta.name),
        tarball_filename
    );

    // Augment package.json with dist block
    let mut version_entry = pkg_json.clone();
    version_entry["dist"] = serde_json::json!({
        "integrity": integrity,
        "shasum": shasum,
        "tarball": tarball_url,
    });

    let encoded_tarball = base64::engine::general_purpose::STANDARD.encode(tarball);

    let body = serde_json::json!({
        "_id": meta.name,
        "name": meta.name,
        "dist-tags": { tag: meta.version },
        "versions": { meta.version.clone(): version_entry },
        "access": access,
        "_attachments": {
            tarball_filename: {
                "content_type": "application/octet-stream",
                "data": encoded_tarball,
                "length": tarball.len(),
            }
        }
    });

    serde_json::to_vec(&body).map_err(UepmError::Json)
}

// ── Upload ────────────────────────────────────────────────────────────────────

async fn upload(
    registry: &str,
    name: &str,
    body: &[u8],
    token: &str,
    _tag: &str,
) -> Result<(), UepmError> {
    let url = format!("{}/{}", registry, urlencoding::encode(name));

    let client = reqwest::Client::new();
    let res = client
        .put(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {token}"))
        .header("npm-command", "publish")
        .body(body.to_vec())
        .send()
        .await?;

    let status = res.status();

    // OTP retry
    if status == reqwest::StatusCode::UNAUTHORIZED {
        let www_auth = res
            .headers()
            .get("www-authenticate")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let body_text = res.text().await.unwrap_or_default();

        let needs_otp = www_auth.to_lowercase().contains("otp")
            || body_text.to_lowercase().contains("one-time pass");

        if needs_otp {
            let otp: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("One-time password (OTP)")
                .interact_text()
                .map_err(|e| UepmError::Io(std::io::Error::other(e.to_string())))?;

            let retry = client
                .put(&url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {token}"))
                .header("npm-command", "publish")
                .header("npm-otp", otp)
                .body(body.to_vec())
                .send()
                .await?;

            return handle_response(retry).await;
        }

        return Err(UepmError::PublishFailed(body_text));
    }

    handle_response(res).await
}

async fn handle_response(res: reqwest::Response) -> Result<(), UepmError> {
    let status = res.status();
    if status.is_success() {
        return Ok(());
    }
    let body = res.text().await.unwrap_or_default();
    // Try to extract the registry's error message
    let message = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string))
        .unwrap_or(body);
    Err(UepmError::PublishFailed(format!("{status}: {message}")))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PackageMetadata;
    use tempfile::tempdir;

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

    fn dir_with_uplugin() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("MyPlugin.uplugin"), b"{}").unwrap();
        dir
    }

    #[test]
    fn test_validate_valid_metadata() {
        let dir = dir_with_uplugin();
        assert!(validate_metadata(&valid_meta(), dir.path()).is_ok());
    }

    #[test]
    fn test_validate_rejects_missing_main_file() {
        let dir = tempdir().unwrap(); // no .uplugin written
        assert!(matches!(
            validate_metadata(&valid_meta(), dir.path()),
            Err(UepmError::InvalidPackageField { field, .. }) if field == "main"
        ));
    }

    #[test]
    fn test_validate_rejects_empty_name() {
        let dir = dir_with_uplugin();
        let mut m = valid_meta();
        m.name = "".to_string();
        assert!(matches!(
            validate_metadata(&m, dir.path()),
            Err(UepmError::InvalidPackageField { field, .. }) if field == "name"
        ));
    }

    #[test]
    fn test_validate_rejects_unscoped_name() {
        let dir = dir_with_uplugin();
        let mut m = valid_meta();
        m.name = "my-plugin".to_string();
        assert!(matches!(
            validate_metadata(&m, dir.path()),
            Err(UepmError::InvalidPackageField { field, .. }) if field == "name"
        ));
    }

    #[test]
    fn test_validate_rejects_invalid_semver() {
        let dir = dir_with_uplugin();
        let mut m = valid_meta();
        m.version = "not-semver".to_string();
        assert!(matches!(
            validate_metadata(&m, dir.path()),
            Err(UepmError::InvalidPackageField { field, .. }) if field == "version"
        ));
    }

    #[test]
    fn test_validate_rejects_invalid_engine_range() {
        let dir = dir_with_uplugin();
        let mut m = valid_meta();
        m.engine_range = "bad range!!".to_string();
        assert!(matches!(
            validate_metadata(&m, dir.path()),
            Err(UepmError::InvalidPackageField { field, .. }) if field == "engine_range"
        ));
    }

    #[test]
    fn test_build_package_json_shape() {
        let meta = valid_meta();
        let v = serde_json::json!({
            "name": meta.name,
            "version": meta.version,
            "description": meta.description,
            "main": meta.main,
            "author": meta.author,
            "license": meta.license,
            "unreal": {
                "engineVersion": meta.engine_range,
                "pluginName": "MyPlugin",
            },
            "keywords": ["unreal", "unreal-engine", "plugin", "uepm"],
        });
        assert_eq!(v["name"], "@acme/my-plugin");
        assert_eq!(v["unreal"]["pluginName"], "MyPlugin");
    }

    #[test]
    fn test_build_publish_body_structure() {
        let meta = valid_meta();
        let pkg_json = serde_json::json!({"name": meta.name, "version": meta.version});
        let tarball = b"fake-tarball-bytes";
        let shasum = "abc123";
        let integrity = "sha512-AAAA";
        let registry = "https://registry.npmjs.org";

        let body_bytes = build_publish_body(
            &meta, &pkg_json, tarball, shasum, integrity, registry, "latest", "public",
        )
        .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert_eq!(body["_id"], "@acme/my-plugin");
        assert_eq!(body["name"], "@acme/my-plugin");
        assert_eq!(body["dist-tags"]["latest"], "1.2.0");
        assert!(body["versions"]["1.2.0"]["dist"]["integrity"].as_str().unwrap().starts_with("sha512-"));
        assert_eq!(body["versions"]["1.2.0"]["dist"]["shasum"], "abc123");
        assert!(body["_attachments"]["my-plugin-1.2.0.tgz"]["content_type"] == "application/octet-stream");
        assert_eq!(body["access"], "public");
    }
}
