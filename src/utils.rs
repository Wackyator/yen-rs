use std::{
    cmp::min,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

use flate2::read::GzDecoder;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use miette::IntoDiagnostic;
use reqwest::header;
use tar::Archive;

use crate::{
    github::{resolve_python_version, Version},
    PYTHON_INSTALLS_PATH, YEN_CLIENT,
};

pub async fn ensure_python(version: Version) -> miette::Result<(Version, PathBuf)> {
    if !PYTHON_INSTALLS_PATH.exists() {
        log::debug!("creating python installs dir");
        let s = PYTHON_INSTALLS_PATH.to_str();
        log::debug!("{s:?}");
        fs::create_dir(&*PYTHON_INSTALLS_PATH).into_diagnostic()?;
        log::debug!("created python installs dir");
    }

    let (version, link) = resolve_python_version(version).await?;
    log::debug!("{version}, {link}");

    let download_dir = PYTHON_INSTALLS_PATH.join(version.to_string());
    log::debug!("{}", download_dir.to_string_lossy());

    let python_bin_path = download_dir.join("python/bin/python3");
    log::debug!("{}", python_bin_path.to_string_lossy());

    if python_bin_path.exists() {
        return Ok((version, python_bin_path));
    }

    log::debug!("creating python bin");
    fs::create_dir_all(&python_bin_path).into_diagnostic()?;

    let downloaded_file =
        File::open(download(link.as_str(), &download_dir).await?).into_diagnostic()?;

    log::debug!("Decoding tar");
    Archive::new(GzDecoder::new(downloaded_file))
        .unpack(download_dir)
        .into_diagnostic()?;

    Ok((version, python_bin_path))
}

pub async fn create_env(
    version: Version,
    python_bin_path: PathBuf,
    venv_path: PathBuf,
) -> miette::Result<()> {
    if venv_path.exists() {
        miette::bail!("Error: {} already exists!", venv_path.to_string_lossy());
    }

    let stdout = Command::new(format!("{}", python_bin_path.to_string_lossy()))
        .args(["-m", "venv", &format!("{}", venv_path.to_string_lossy())])
        .output()
        .into_diagnostic()?;

    if !stdout.status.success() {
        miette::bail!("Error: unable to create venv!");
    }

    eprintln!(
        "Created {} with Python {}",
        venv_path.to_string_lossy(),
        version
    );

    Ok(())
}

pub async fn download(link: &str, path: &Path) -> miette::Result<PathBuf> {
    let filepath = path.join(
        link.split('/')
            .collect::<Vec<_>>()
            .last()
            .ok_or(miette::miette!("Unable to file name from link"))?,
    );
    log::debug!("{}", filepath.to_string_lossy());

    let resource = YEN_CLIENT.get(link).send().await.into_diagnostic()?;
    let total_size = resource.content_length().ok_or(miette::miette!(
        "Failed to get content length from '{link}'"
    ))?;

    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})"
            )
            .into_diagnostic()?
            .progress_chars("#>-")
    );
    pb.set_message(format!("Downloading {link}"));

    log::debug!("creating file");
    let mut file = File::create(filepath).into_diagnostic()?;
    let mut downloaded: u64 = 0;
    let mut stream = resource.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.into_diagnostic()?;
        file.write_all(&chunk).into_diagnostic()?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Downloaded {link}"));

    Ok(path.join("shit"))
}

pub fn yen_client() -> reqwest::Client {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("YenClient"),
    );

    match reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .into_diagnostic()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
