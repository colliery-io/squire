//! Server startup-pull of the phone APK from the public release repo (SQUIRE-T-0087 / ADR
//! SQUIRE-A-0012, SQUIRE-A-0018).
//!
//! On startup the server fetches the **latest GitHub Release** of `colliery-io/squire` (public — no
//! auth), finds the `squire-<versionCode>.apk` asset, downloads it into the OTA updates dir, and
//! writes the `manifest.json` the LAN `/app/*` endpoints serve (the server injects the sha256,
//! SQUIRE-T-0085). It then refreshes on an interval so a long-running server keeps current.
//!
//! **Best-effort**: any failure (offline, no release, parse error) is logged and leaves whatever is
//! already present, so the server still serves its existing APK. It never blocks startup or panics.

use std::path::{Path, PathBuf};
use std::time::Duration;

/// The public repo whose Releases carry the APK and server binaries (SQUIRE-A-0018). Also the
/// self-update default ([`crate::updater`]). Overridable via `SQUIRE_DIST_REPO`.
pub(crate) const DEFAULT_REPO: &str = "colliery-io/squire";
const REFRESH: Duration = Duration::from_secs(6 * 60 * 60); // 6h

/// Spawn the background APK sync: an immediate pull, then a refresh loop. No-op (logs) when
/// `SQUIRE_APK_SYNC=off`. Repo overridable via `SQUIRE_DIST_REPO` (`owner/name`).
pub fn spawn_apk_sync(updates_dir: PathBuf) {
    if std::env::var("SQUIRE_APK_SYNC").is_ok_and(|v| v.eq_ignore_ascii_case("off")) {
        tracing::info!("apk-sync: off (SQUIRE_APK_SYNC=off)");
        return;
    }
    let repo = std::env::var("SQUIRE_DIST_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_string());
    tracing::info!(repo = %repo, dir = %updates_dir.display(), "apk-sync: syncing from GitHub");

    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .user_agent("squire-serve")
            .timeout(Duration::from_secs(60))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "apk-sync: could not build http client");
                return;
            }
        };
        loop {
            match sync_once(&client, &repo, &updates_dir).await {
                Ok(Some(file)) => tracing::info!(file = %file, "apk-sync: now serving"),
                Ok(None) => {} // up to date / nothing to do
                Err(e) => {
                    tracing::warn!(error = %e, "apk-sync: failed (serving whatever is already present)")
                }
            }
            tokio::time::sleep(REFRESH).await;
        }
    });
}

/// One pull. Returns the newly-installed file name, or `None` when already current. Errors are
/// caller-logged; they never abort the loop.
async fn sync_once(
    client: &reqwest::Client,
    repo: &str,
    dir: &Path,
) -> Result<Option<String>, String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    // Authenticate when a token is set (SQUIRE-T-0117) to dodge the 60 req/hr unauthenticated 403s.
    let mut req = client
        .get(&url)
        .header("Accept", "application/vnd.github+json");
    if let Some(tok) = crate::updater::update_token() {
        req = req.header("Authorization", format!("Bearer {tok}"));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("fetch latest release: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("latest release: HTTP {}", resp.status()));
    }
    let rel: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("parse release json: {e}"))?;

    let tag = rel
        .get("tag_name")
        .and_then(|t| t.as_str())
        .unwrap_or_default();
    let version_name = tag.strip_prefix('v').unwrap_or(tag).to_string();

    let assets = rel
        .get("assets")
        .and_then(|a| a.as_array())
        .ok_or("release has no assets")?;
    let (file, dl_url, version_code) = assets
        .iter()
        .find_map(|a| {
            let name = a.get("name")?.as_str()?;
            let code = parse_version_code(name)?;
            let dl = a.get("browser_download_url")?.as_str()?;
            Some((name.to_string(), dl.to_string(), code))
        })
        .ok_or("no squire-<versionCode>.apk asset in the latest release")?;

    let target = dir.join(&file);
    if target.is_file() {
        // Idempotent: already have this build — just make sure the manifest points at it.
        write_manifest(dir, &file, version_code, &version_name)?;
        return Ok(None);
    }

    let bytes = client
        .get(&dl_url)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("download {file}: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("read {file}: {e}"))?;

    // Write to a temp then rename so a half-written APK is never served.
    let tmp = dir.join(format!(".{file}.part"));
    std::fs::write(&tmp, &bytes).map_err(|e| format!("write {file}: {e}"))?;
    std::fs::rename(&tmp, &target).map_err(|e| format!("install {file}: {e}"))?;
    write_manifest(dir, &file, version_code, &version_name)?;
    tracing::info!(
        file = %file,
        version = %version_name,
        bytes = bytes.len(),
        "apk-sync: pulled new build",
    );
    Ok(Some(file))
}

/// `squire-<digits>.apk` → the version code; `None` for anything else (e.g. server binaries).
fn parse_version_code(name: &str) -> Option<u32> {
    name.strip_prefix("squire-")?
        .strip_suffix(".apk")?
        .parse()
        .ok()
}

fn write_manifest(
    dir: &Path,
    file: &str,
    version_code: u32,
    version_name: &str,
) -> Result<(), String> {
    let manifest = serde_json::json!({
        "squire": { "versionCode": version_code, "versionName": version_name, "file": file }
    });
    let body = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("manifest.json"), body).map_err(|e| format!("write manifest: {e}"))
}

#[cfg(test)]
mod tests {
    use super::parse_version_code;

    #[test]
    fn parses_apk_names() {
        assert_eq!(parse_version_code("squire-6.apk"), Some(6));
        assert_eq!(parse_version_code("squire-123.apk"), Some(123));
        assert_eq!(parse_version_code("squire-serve-macos"), None);
        assert_eq!(parse_version_code("squire-.apk"), None);
        assert_eq!(parse_version_code("squire-6.apk.bak"), None);
    }
}
