//! LAN app-update distribution (SQUIRE-T-0051): serve a version manifest + the signed APKs from a
//! directory (`SQUIRE_APK_DIR`) so paired phones can self-update over the home network.
//!
//! **Unauthenticated** by design — the endpoints are LAN-only and the APKs/manifest aren't secret,
//! and the in-app "Get update" flow opens the download URL in the phone's browser (which can't carry
//! a bearer token). A missing/unset dir yields an empty manifest (the app simply finds no update),
//! never a 500. The download route is path-traversal-safe: only a bare `<name>.apk` from the dir.

use std::collections::HashMap;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};

/// The configured APK directory, only if it's set and actually a directory.
fn apk_dir() -> Option<PathBuf> {
    std::env::var_os("SQUIRE_APK_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
}

/// Lower-hex SHA-256 of a file, memoised by (path, mtime, len) so the common no-change poll doesn't
/// re-read the APK every time. Returns None if the file can't be read.
fn apk_sha256(path: &FsPath) -> Option<String> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, (SystemTime, u64, String)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));

    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let len = meta.len();
    if let Ok(map) = cache.lock() {
        if let Some((m, l, h)) = map.get(path) {
            if *m == mtime && *l == len {
                return Some(h.clone());
            }
        }
    }

    let bytes = std::fs::read(path).ok()?;
    let digest = Sha256::digest(&bytes);
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if let Ok(mut map) = cache.lock() {
        map.insert(path.to_path_buf(), (mtime, len, hex.clone()));
    }
    Some(hex)
}

/// `GET /app/manifest` — the admin-maintained `manifest.json` from `SQUIRE_APK_DIR`, enriched with a
/// server-computed `sha256` per release (so content-hash update detection can't drift from the file;
/// SQUIRE-T-0085). `{}` when the dir is unset/absent (so the apps find no update and show no banner).
pub async fn manifest() -> Response {
    let dir = apk_dir();
    let raw = dir
        .as_ref()
        .map(|d| d.join("manifest.json"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_else(|| "{}".to_string());

    // Parse + inject sha256 for every release whose `file` exists in the dir. On any parse failure
    // fall back to serving the raw body verbatim — never a 500.
    let body = match (
        dir.as_ref(),
        serde_json::from_str::<serde_json::Value>(&raw),
    ) {
        (Some(dir), Ok(mut v)) => {
            if let Some(obj) = v.as_object_mut() {
                for release in obj.values_mut() {
                    let file = release
                        .get("file")
                        .and_then(|f| f.as_str())
                        .map(str::to_owned);
                    if let Some(file) = file {
                        if let Some(hash) = apk_sha256(&dir.join(&file)) {
                            if let Some(r) = release.as_object_mut() {
                                r.insert("sha256".to_string(), serde_json::Value::String(hash));
                            }
                        }
                    }
                }
            }
            serde_json::to_string(&v).unwrap_or(raw)
        }
        _ => raw,
    };

    ([(header::CONTENT_TYPE, "application/json")], body).into_response()
}

/// `GET /app/{file}` — stream a signed APK from `SQUIRE_APK_DIR`. Only a bare `<name>.apk` file
/// name is accepted (no separators / `..`), and only if it exists directly in the dir.
pub async fn download(Path(file): Path<String>) -> Response {
    if file.contains('/') || file.contains('\\') || file.contains("..") || !file.ends_with(".apk") {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let Some(dir) = apk_dir() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let path = dir.join(&file);
    // Belt-and-suspenders: the resolved file must sit directly in the dir.
    if !path.is_file() || path.parent() != Some(dir.as_path()) {
        return StatusCode::NOT_FOUND.into_response();
    }
    match std::fs::read(&path) {
        Ok(bytes) => (
            [(
                header::CONTENT_TYPE,
                "application/vnd.android.package-archive",
            )],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
