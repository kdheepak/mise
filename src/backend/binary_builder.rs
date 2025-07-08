use crate::backend::backend_type::BackendType;
use crate::cli::args::BackendArg;
use crate::cmd::CmdLineRunner;
use crate::config::Settings;
use crate::install_context::InstallContext;
use crate::timeout;
use crate::toolset::ToolVersion;
use crate::{backend::Backend, config::Config};
use async_trait::async_trait;
use std::{fmt::Debug, sync::Arc};
use xx::regex;

#[derive(Debug)]
pub struct BinaryBuilderBackend {
    ba: Arc<BackendArg>,
}

#[async_trait]
impl Backend for BinaryBuilderBackend {
    fn get_type(&self) -> BackendType {
        BackendType::BinaryBuilder
    }

    fn ba(&self) -> &Arc<BackendArg> {
        &self.ba
    }

    fn get_dependencies(&self) -> eyre::Result<Vec<&str>> {
        Ok(vec!["curl", "tar", "gzip"])
    }

    // downloads binaries from Julia's BinaryBuilder github organization

    // downloads binaries from Julia's BinaryBuilder github organization
    async fn list_remote_versions(&self, _config: &Arc<Config>) -> eyre::Result<Vec<String>> {
        // Extract the package name from the backend argument
        let package_name = self.ba.short.as_str();

        // BinaryBuilder packages typically have a releases.json or similar manifest
        // The URL pattern for BinaryBuilder packages is usually:
        // https://github.com/JuliaBinaryWrappers/{package_name}_jll.jl/releases/download/{version}/Artifacts.toml
        // But for listing versions, we need to check the GitHub API or a manifest file

        // Option 1: Use GitHub API to list releases
        let github_api_url = format!(
            "https://api.github.com/repos/JuliaBinaryWrappers/{}_jll.jl/releases",
            package_name
        );

        // Make HTTP request to get releases
        let response = crate::http::HTTP.get_text(&github_api_url).await?;

        // Parse GitHub releases JSON
        let releases: Vec<serde_json::Value> = serde_json::from_str(&response)?;

        // Extract version numbers from release tags
        let mut versions = Vec::new();
        for release in releases {
            if let Some(tag_name) = release.get("tag_name").and_then(|v| v.as_str()) {
                // BinaryBuilder tags often have a format like "PackageName-v1.2.3+0"
                // We need to extract just the version part
                if let Some(version) = extract_version_from_tag(tag_name) {
                    versions.push(version);
                }
            }
        }

        // Sort versions in descending order (newest first)
        versions.sort_by(
            |a, b| match (semver::Version::parse(a), semver::Version::parse(b)) {
                (Ok(va), Ok(vb)) => vb.cmp(&va),
                _ => b.cmp(a),
            },
        );

        Ok(versions)
    }
}

impl BinaryBuilderBackend {
    pub fn from_arg(ba: BackendArg) -> Self {
        Self { ba: Arc::new(ba) }
    }
}

fn trim_after_last_slash(s: String) -> Option<String> {
    s.rsplit_once('/').map(|(new_path, _)| new_path.to_string())
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BinaryBuilderModInfo {
    versions: Vec<String>,
}
