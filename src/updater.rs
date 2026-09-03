use std::io::{Read, Write};
use std::sync::mpsc::Sender;

pub const GITHUB_RELEASE_URL: &str =
    "https://api.github.com/repos/TrulyScarlet/Epod/releases/tags/nightly";
pub const CURRENT_COMMIT_SHA: &str = env!("EPOD_COMMIT_SHA");

pub fn get_target_asset_name() -> Result<&'static str, &'static str> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return Ok("epod-x86_64-pc-windows-msvc.zip");

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return Ok("epod-x86_64-apple-darwin.zip");

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return Ok("epod-aarch64-apple-darwin.zip");

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return Ok("epod-x86_64-unknown-linux-gnu.zip");

    #[allow(unreachable_code)]
    Err("Unsupported OS or CPU architecture for automated updates")
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: Option<u64>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, Debug, Clone)]
pub struct GithubRelease {
    pub tag_name: String,
    pub target_commitish: String,
    pub assets: Vec<ReleaseAsset>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    UpdateAvailable {
        remote_sha: String,
        download_url: String,
    },
    Downloading(f32),
    InstalledRestartRequired,
    Error(String),
}

pub fn check_for_updates() -> Result<UpdateStatus, Box<dyn std::error::Error + Send + Sync>> {
    let target_asset_name = get_target_asset_name()?;
    let resp: GithubRelease = ureq::get(GITHUB_RELEASE_URL)
        .set("User-Agent", "Epod-Updater")
        .call()?
        .into_json()?;

    let asset = resp
        .assets
        .iter()
        .find(|a| a.name == target_asset_name)
        .ok_or_else(|| format!("Asset '{}' not found in nightly release", target_asset_name))?;

    let remote_sha = resp.target_commitish.trim().to_string();
    let current_sha = CURRENT_COMMIT_SHA.trim();

    if current_sha == "dev" {
        // Dev build: allow triggering test update
        Ok(UpdateStatus::UpdateAvailable {
            remote_sha: format!("{} (dev build)", &remote_sha[..remote_sha.len().min(7)]),
            download_url: asset.browser_download_url.clone(),
        })
    } else if current_sha.eq_ignore_ascii_case(&remote_sha)
        || (!remote_sha.is_empty() && current_sha.starts_with(&remote_sha))
        || (!current_sha.is_empty() && remote_sha.starts_with(current_sha))
    {
        Ok(UpdateStatus::UpToDate)
    } else {
        Ok(UpdateStatus::UpdateAvailable {
            remote_sha,
            download_url: asset.browser_download_url.clone(),
        })
    }
}

pub fn download_and_apply_update(
    download_url: &str,
    on_progress: impl Fn(f32) + Send + 'static,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp = ureq::get(download_url)
        .set("User-Agent", "Epod-Updater")
        .call()?;

    let total_size = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok());

    let mut temp_zip = tempfile::NamedTempFile::new()?;
    let mut reader = resp.into_reader();
    let mut buffer = [0u8; 16384];
    let mut downloaded: u64 = 0;

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        temp_zip.write_all(&buffer[..n])?;
        downloaded += n as u64;
        if let Some(total) = total_size {
            if total > 0 {
                let progress = (downloaded as f32 / total as f32).min(1.0);
                on_progress(progress);
            }
        }
    }
    temp_zip.flush()?;
    on_progress(1.0);

    let mut archive = zip::ZipArchive::new(temp_zip.reopen()?)?;
    let mut exe_index = None;
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let name = file.name().to_lowercase();
        if name == "epod"
            || name == "epod.exe"
            || name.ends_with("/epod")
            || name.ends_with("/epod.exe")
            || name.ends_with("\\epod.exe")
            || name.ends_with("\\epod")
        {
            exe_index = Some(i);
            break;
        }
    }

    let exe_index = exe_index.ok_or("Binary 'epod' or 'epod.exe' not found inside update archive")?;
    let mut exe_file = archive.by_index(exe_index)?;

    let current_exe = std::env::current_exe()?;
    let current_dir = current_exe
        .parent()
        .ok_or("Cannot determine current executable directory")?;

    let staged_exe_path = current_dir.join(format!(".epod_update_{}.tmp", std::process::id()));
    {
        let mut staged_file = std::fs::File::create(&staged_exe_path)?;
        std::io::copy(&mut exe_file, &mut staged_file)?;
        staged_file.sync_all()?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(&staged_exe_path, permissions)?;
        }
    }

    let replace_res = self_replace::self_replace(&staged_exe_path);
    let _ = std::fs::remove_file(&staged_exe_path);

    replace_res?;
    Ok(())
}

pub fn spawn_update_check(sender: Sender<UpdateStatus>) {
    let _ = sender.send(UpdateStatus::Checking);
    std::thread::spawn(move || {
        let status = match check_for_updates() {
            Ok(status) => status,
            Err(e) => UpdateStatus::Error(e.to_string()),
        };
        let _ = sender.send(status);
    });
}

pub fn spawn_download_and_apply(download_url: String, sender: Sender<UpdateStatus>) {
    let _ = sender.send(UpdateStatus::Downloading(0.0));
    std::thread::spawn(move || {
        let progress_tx = sender.clone();
        let res = download_and_apply_update(&download_url, move |p| {
            let _ = progress_tx.send(UpdateStatus::Downloading(p));
        });

        match res {
            Ok(()) => {
                let _ = sender.send(UpdateStatus::InstalledRestartRequired);
            }
            Err(e) => {
                let _ = sender.send(UpdateStatus::Error(e.to_string()));
            }
        }
    });
}
