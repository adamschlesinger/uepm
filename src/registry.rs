use crate::errors::UepmError;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PackageMetadata {
    pub version: String,
    pub tarball: String,
    pub integrity: String,
}

#[derive(Deserialize)]
struct NpmPackage {
    #[serde(rename = "dist-tags")]
    dist_tags: HashMap<String, String>,
    versions: HashMap<String, NpmVersion>,
}

#[derive(Deserialize)]
struct NpmVersion {
    version: String,
    dist: NpmDist,
}

#[derive(Deserialize)]
struct NpmDist {
    tarball: String,
    integrity: String,
}

pub struct RegistryClient {
    base_url: String,
    token: Option<String>,
    client: reqwest::Client,
}

impl RegistryClient {
    pub fn new(base_url: &str, token: Option<String>) -> Self {
        RegistryClient {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Self {
        let base_url = std::env::var("UEPM_REGISTRY")
            .unwrap_or_else(|_| "https://registry.npmjs.org".to_string());
        let token = std::env::var("UEPM_TOKEN").ok();
        Self::new(&base_url, token)
    }

    pub async fn fetch_metadata(&self, package: &str) -> Result<PackageMetadata, UepmError> {
        let encoded = urlencoding::encode(package).into_owned();
        let url = format!("{}/{}", self.base_url, encoded);

        tracing::debug!("Fetching metadata from {url}");

        let mut req = self.client.get(&url);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }

        let resp = req.send().await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(UepmError::PackageNotFound {
                package: package.to_string(),
            });
        }
        resp.error_for_status_ref()?;

        let npm_pkg: NpmPackage = resp.json().await?;

        let latest = npm_pkg
            .dist_tags
            .get("latest")
            .ok_or_else(|| UepmError::PackageNotFound {
                package: package.to_string(),
            })?;

        let version_info =
            npm_pkg
                .versions
                .get(latest)
                .ok_or_else(|| UepmError::PackageNotFound {
                    package: package.to_string(),
                })?;

        Ok(PackageMetadata {
            version: version_info.version.clone(),
            tarball: version_info.dist.tarball.clone(),
            integrity: version_info.dist.integrity.clone(),
        })
    }

    pub async fn fetch_metadata_for_version(
        &self,
        package: &str,
        range: &str,
    ) -> Result<PackageMetadata, UepmError> {
        let encoded = urlencoding::encode(package).into_owned();
        let url = format!("{}/{}", self.base_url, encoded);

        let mut req = self.client.get(&url);
        if let Some(ref token) = self.token {
            req = req.bearer_auth(token);
        }

        let resp = req.send().await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(UepmError::PackageNotFound {
                package: package.to_string(),
            });
        }
        resp.error_for_status_ref()?;

        let npm_pkg: NpmPackage = resp.json().await?;
        let versions: Vec<String> = npm_pkg.versions.keys().cloned().collect();
        let best = resolve_best_version(&versions, range)?;

        let version_info = npm_pkg.versions.get(&best).unwrap();

        Ok(PackageMetadata {
            version: version_info.version.clone(),
            tarball: version_info.dist.tarball.clone(),
            integrity: version_info.dist.integrity.clone(),
        })
    }
}

pub fn resolve_best_version(versions: &[String], range: &str) -> Result<String, UepmError> {
    let req = semver::VersionReq::parse(range).map_err(|e| UepmError::InvalidSemver {
        range: range.to_string(),
        message: e.to_string(),
    })?;

    let mut matching: Vec<semver::Version> = versions
        .iter()
        .filter_map(|v| semver::Version::parse(v).ok())
        .filter(|v| req.matches(v))
        .collect();

    matching.sort();

    matching
        .into_iter()
        .last()
        .map(|v| v.to_string())
        .ok_or_else(|| UepmError::NoMatchingVersion {
            package: String::new(),
            range: range.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    fn sample_registry_response(pkg: &str, version: &str, tarball_url: &str) -> serde_json::Value {
        serde_json::json!({
            "name": pkg,
            "dist-tags": { "latest": version },
            "versions": {
                version: {
                    "name": pkg,
                    "version": version,
                    "dist": {
                        "tarball": tarball_url,
                        "integrity": "sha512-47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
                    }
                }
            }
        })
    }

    #[tokio::test]
    async fn test_fetch_metadata_success() {
        let mut server = Server::new_async().await;
        let pkg = "@acme/cool-plugin";
        let encoded = "%40acme%2Fcool-plugin";
        let tarball_url = format!("{}/tarball.tgz", server.url());
        let body = sample_registry_response(pkg, "1.0.3", &tarball_url);

        let _mock = server
            .mock("GET", format!("/{encoded}").as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create_async()
            .await;

        let client = RegistryClient::new(&server.url(), None);
        let meta = client.fetch_metadata(pkg).await.unwrap();
        assert_eq!(meta.version, "1.0.3");
        assert_eq!(meta.tarball, tarball_url);
    }

    #[tokio::test]
    async fn test_fetch_metadata_not_found() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("GET", "/%40acme%2Fmissing")
            .with_status(404)
            .create_async()
            .await;

        let client = RegistryClient::new(&server.url(), None);
        let result = client.fetch_metadata("@acme/missing").await;
        assert!(matches!(result, Err(UepmError::PackageNotFound { .. })));
    }

    #[test]
    fn test_resolve_best_version() {
        let versions = vec!["1.0.0".to_string(), "1.0.3".to_string(), "2.0.0".to_string()];
        let best = resolve_best_version(&versions, "^1.0.0").unwrap();
        assert_eq!(best, "1.0.3");
    }

    #[test]
    fn test_resolve_no_match() {
        let versions = vec!["2.0.0".to_string()];
        let result = resolve_best_version(&versions, "^1.0.0");
        assert!(result.is_err());
    }
}
