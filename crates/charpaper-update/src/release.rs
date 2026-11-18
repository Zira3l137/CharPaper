use std::time::Duration;

use semver::Version;
use serde::Deserialize;
use ureq::Agent;

use crate::Error;
use crate::Platform;
use crate::REPOSITORY;
use crate::platform;

const SUMS_FILE: &str = "SHA256SUMS.txt";

// A newer stable version, with what's needed to download it.
#[derive(Debug, Clone)]
pub struct Release {
    pub version: Version,
    // Its page on GitHub, with the notes.
    pub page: String,
    pub notes: String,
    pub(crate) tag: String,
    pub(crate) archive: Asset,
    pub(crate) sums: Asset,
}

#[derive(Debug, Clone)]
pub(crate) struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

// GitHub's "latest" is the newest release that is neither a draft nor a pre-release, so the
// nightly and release candidates never show up here. None when it is no newer than `current`.
pub fn check(current: &Version) -> Result<Option<Release>, Error> {
    let platform = platform().ok_or(Error::Unsupported)?;
    let url = format!("https://api.github.com/repos/{REPOSITORY}/releases/latest");
    let text = agent()
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()?
        .body_mut()
        .read_to_string()?;
    let latest: GitHubRelease = serde_json::from_str(&text)?;
    let Some(version) = newer(current, &latest.tag_name) else {
        return Ok(None);
    };
    let find = |name: &str| {
        let asset = latest.assets.iter().find(|asset| asset.name == name)?;
        Some(Asset {
            name: asset.name.clone(),
            url: asset.browser_download_url.clone(),
            size: asset.size,
        })
    };
    let archive = archive_name(platform, &latest.tag_name);
    let no_archive = || Error::NoArchive(latest.tag_name.clone());
    Ok(Some(Release {
        version,
        page: latest.html_url.clone(),
        notes: latest.body.clone().unwrap_or_default(),
        archive: find(&archive).ok_or_else(no_archive)?,
        sums: find(SUMS_FILE).ok_or_else(no_archive)?,
        tag: latest.tag_name,
    }))
}

// Downloads can take minutes on a slow line, so only a stalled read or connection times out,
// never the transfer as a whole.
pub(crate) fn agent() -> Agent {
    Agent::config_builder()
        .user_agent(concat!("CharPaper/", env!("CARGO_PKG_VERSION")))
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_per_read(Some(Duration::from_secs(30)))
        .build()
        .new_agent()
}

// Tags are `v` and a semver version. A pre-release one is skipped even if GitHub were to
// call it latest.
fn newer(current: &Version, tag: &str) -> Option<Version> {
    let version = Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()?;
    (version.pre.is_empty() && version > *current).then_some(version)
}

fn archive_name(platform: &Platform, tag: &str) -> String {
    format!("charpaper-{tag}-{}.{}", platform.name, platform.archive.extension())
}

// sha256sum's lines: the hash, then a space and either a space or `*`, then the name.
pub(crate) fn checksum_for<'a>(sums: &'a str, file: &str) -> Option<&'a str> {
    sums.lines().find_map(|line| {
        let (hash, name) = line.trim_end().split_once(' ')?;
        let name = name.strip_prefix([' ', '*']).unwrap_or(name);
        (name == file).then_some(hash)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_newer_stable_versions() {
        let current = Version::new(0, 6, 0);
        assert_eq!(newer(&current, "v0.7.0"), Some(Version::new(0, 7, 0)));
        assert_eq!(newer(&current, "v0.6.1"), Some(Version::new(0, 6, 1)));
        assert_eq!(newer(&current, "v0.6.0"), None);
        assert_eq!(newer(&current, "v0.5.9"), None);
        assert_eq!(newer(&current, "v0.7.0-rc.1"), None);
        assert_eq!(newer(&current, "nightly"), None);
    }

    #[test]
    fn archive_names_match_build_yml() {
        let windows = &crate::PLATFORMS[0];
        assert_eq!(archive_name(windows, "v0.7.0"), "charpaper-v0.7.0-windows-x64.zip");
    }

    #[test]
    fn checksums_in_text_and_binary_form() {
        let sums = "aaa  charpaper-v0.7.0-windows-x64.zip\nbbb *charpaper_exporter-v0.7.0.zip\r\n";
        assert_eq!(checksum_for(sums, "charpaper-v0.7.0-windows-x64.zip"), Some("aaa"));
        assert_eq!(checksum_for(sums, "charpaper_exporter-v0.7.0.zip"), Some("bbb"));
        assert_eq!(checksum_for(sums, "other.zip"), None);
    }
}
