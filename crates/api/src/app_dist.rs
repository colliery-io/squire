//! LAN app-update distribution (SQUIRE-T-0051): serve a version manifest + the signed APKs from a
//! directory (`SQUIRE_APK_DIR`) so paired phones can self-update over the home network.
//!
//! **Unauthenticated** by design — the endpoints are LAN-only and the APKs/manifest aren't secret,
//! and the in-app "Get update" flow opens the download URL in the phone's browser (which can't carry
//! a bearer token). A missing/unset dir yields an empty manifest (the app simply finds no update),
//! never a 500. The download route is path-traversal-safe: only a bare `<name>.apk` from the dir.

use std::path::PathBuf;

use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

/// The configured APK directory, only if it's set and actually a directory.
fn apk_dir() -> Option<PathBuf> {
    std::env::var_os("SQUIRE_APK_DIR").map(PathBuf::from).filter(|p| p.is_dir())
}

/// `GET /app/manifest` — the admin-maintained `manifest.json` from `SQUIRE_APK_DIR`, or `{}` when
/// the dir is unset/absent (so the apps find no update and show no banner).
pub async fn manifest() -> Response {
    let body = apk_dir()
        .map(|d| d.join("manifest.json"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_else(|| "{}".to_string());
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
            [(header::CONTENT_TYPE, "application/vnd.android.package-archive")],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
