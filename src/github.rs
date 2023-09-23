use std::{collections::BTreeMap, fmt::Display, str::FromStr};

use detect_targets::detect_targets;
use lazy_static::lazy_static;
use miette::IntoDiagnostic;
use regex::Regex;
use reqwest::header::USER_AGENT;
use serde::Deserialize;

const GITHUB_API_URL: &str =
    "https://api.github.com/repos/indygreg/python-build-standalone/releases/latest";

lazy_static! {
    static ref RE: Regex = Regex::new(r"cpython-(\d+\.\d+.\d+)").expect("Unable to create regex!");
}

#[derive(Clone, Debug, Deserialize)]
pub struct GithubResp {
    assets: Vec<Asset>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Asset {
    browser_download_url: String,
}

impl From<GithubResp> for Vec<String> {
    fn from(value: GithubResp) -> Self {
        value
            .assets
            .into_iter()
            .map(|asset| asset.browser_download_url)
            .collect::<Vec<_>>()
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Hash, PartialOrd, Ord)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
}

impl FromStr for Version {
    type Err = miette::ErrReport;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let v = s
            .split('.')
            .map(|s| s.parse::<u32>().into_diagnostic())
            .collect::<Result<Vec<_>, miette::ErrReport>>()?;

        Ok(Self {
            major: v[0],
            minor: v[1],
            patch: v[2],
        })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub enum MachineSuffix {
    DarwinArm64,
    DarwinX64,
    LinuxAarch64,
    LinuxX64GlibC,
    LinuxX64Musl,
}

impl MachineSuffix {
    fn get_suffix(&self) -> String {
        match self {
            Self::DarwinArm64 => "aarch64-apple-darwin-install_only.tar.gz".into(),
            Self::DarwinX64 => "x86_64-apple-darwin-install_only.tar.gz".into(),
            Self::LinuxAarch64 => "aarch64-unknown-linux-gnu-install_only.tar.gz".into(),
            Self::LinuxX64GlibC => "x86_64_v3-unknown-linux-gnu-install_only.tar.gz".into(),
            Self::LinuxX64Musl => "x86_64_v3-unknown-linux-musl-install_only.tar.gz".into(),
        }
    }

    async fn default() -> miette::Result<Self> {
        match detect_targets().await[0].as_str() {
            "x86_64-unknown-linux-musl" => Ok(Self::LinuxX64Musl),
            "x86_64-unknown-linux-gnu" => Ok(Self::LinuxX64GlibC),
            "aarch64-unknown-linux-gnu" => Ok(Self::LinuxAarch64),
            "aarch64-apple-darwin" => Ok(Self::DarwinArm64),
            "x86_64-apple-darwin" => Ok(Self::DarwinX64),
            _ => miette::bail!("Unknown target!"),
        }
    }
}

async fn get_latest_python_release() -> miette::Result<Vec<String>> {
    Ok(reqwest::Client::new()
        .get(GITHUB_API_URL)
        .header(USER_AGENT, "yen client")
        .send()
        .await
        .into_diagnostic()?
        .json::<GithubResp>()
        .await
        .into_diagnostic()?
        .into())
}

pub async fn list_pythons() -> miette::Result<BTreeMap<Version, String>> {
    let machine_suffix = MachineSuffix::default().await?.get_suffix();

    let releases = get_latest_python_release().await?;

    let mut map = BTreeMap::new();

    for release in releases {
        if release.ends_with(&machine_suffix) {
            let x = RE.captures(&release);
            if let Some(v) = x {
                let version = Version::from_str(&v[1])?;
                map.insert(version, release);
            }
        }
    }

    Ok(map)
}
