#[derive(Debug, thiserror::Error)]
pub enum UepmError {
    #[error("Registry error: {0}")]
    Registry(#[from] reqwest::Error),

    #[error("Package not found: {package}")]
    PackageNotFound { package: String },

    #[error("Checksum mismatch for {package}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        package: String,
        expected: String,
        actual: String,
    },

    #[error("Version conflict for {package}: {message}\nHint: pin a version in your uepm.ini")]
    VersionConflict { package: String, message: String },

    #[error("No .uproject file found in {directory}")]
    UprojectNotFound { directory: String },

    #[error("Failed to parse uepm.ini: {0}")]
    ManifestParse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid semver range '{range}': {message}")]
    InvalidSemver { range: String, message: String },

    #[error("No version of {package} satisfies range {range}")]
    NoMatchingVersion { package: String, range: String },

    #[error("Interactive terminal required. Run with --yes to use detected defaults.")]
    InteractiveRequired,

    #[error("No [Plugin] section found in Config/UEPM.ini. Run 'uepm init' first.")]
    NoPackageMetadata,

    #[error("Invalid value for '{field}': {message}")]
    InvalidPackageField { field: String, message: String },

    #[error("Publish failed: {0}")]
    PublishFailed(String),

    #[error("UEPM_TOKEN is not set. Export it before publishing.")]
    TokenRequired,
}
