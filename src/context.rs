use crate::errors::UepmError;
use crate::registry::RegistryClient;
use std::path::PathBuf;

/// Resolved project context constructed once at startup and passed to every command.
pub struct UEPMContext {
    pub project_dir: PathBuf,
    /// `<project_dir>/UEPMPlugins`
    pub uepm_plugins_dir: PathBuf,
    pub registry: RegistryClient,
    pub token: Option<String>,
}

impl UEPMContext {
    /// Build context from the current working directory and environment variables.
    /// Reads `UEPM_REGISTRY` and `UEPM_TOKEN`.
    pub fn new() -> Result<Self, UepmError> {
        Ok(Self::with_dir(std::env::current_dir()?))
    }

    /// Build context from an explicit project directory. Useful in tests.
    pub fn with_dir(project_dir: PathBuf) -> Self {
        let uepm_plugins_dir = project_dir.join("UEPMPlugins");
        let registry = RegistryClient::from_env();
        let token = std::env::var("UEPM_TOKEN").ok();
        Self { project_dir, uepm_plugins_dir, registry, token }
    }

    /// Build context with explicit registry URL and token — no env var reads.
    /// Use this in integration tests to avoid races between parallel test threads.
    pub fn for_test(project_dir: PathBuf, registry_url: &str, token: Option<String>) -> Self {
        let uepm_plugins_dir = project_dir.join("UEPMPlugins");
        let registry = RegistryClient::new(registry_url, token.clone());
        Self { project_dir, uepm_plugins_dir, registry, token }
    }
}
