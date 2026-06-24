//! "Install on a new phone" QR (SQUIRE-T-0088 / ADR SQUIRE-A-0012): render the LAN download URL for
//! the currently-published phone APK as a QR a new device can scan to download + install. The APK
//! itself is served by the LAN api at `GET /app/<file>` (unauthenticated, SQUIRE-T-0051); here we
//! just point a QR at it, reusing the Keep's existing QR rendering + advertised LAN address.

use std::path::PathBuf;

use axum::Json;
use qrcode::render::svg;
use qrcode::QrCode;
use serde::Serialize;

use crate::pair::advertised_addr;
use crate::Operator;

/// The current installable build (the LAN URL + QR), or `available: false` when nothing is published.
#[derive(Debug, Default, Serialize)]
pub struct AppInstallView {
    pub available: bool,
    pub version_name: Option<String>,
    pub version_code: Option<u64>,
    pub file: Option<String>,
    /// The LAN URL a phone downloads from, e.g. `http://10.0.0.227:8088/app/squire-6.apk`.
    pub url: Option<String>,
    /// A self-contained SVG of the QR, for the page to drop straight into the DOM.
    pub qr_svg: Option<String>,
}

/// `GET /api/app/install` (Knight-only) — the LAN install URL + QR for the current phone APK, or
/// `available: false` when no build is published / the file is missing. Reads the same
/// `SQUIRE_APK_DIR/manifest.json` the LAN api serves (kept current by the startup-pull, T-0087).
pub async fn app_install(Operator(_op): Operator) -> Json<AppInstallView> {
    Json(install_view().unwrap_or_default())
}

fn install_view() -> Option<AppInstallView> {
    let dir = std::env::var_os("SQUIRE_APK_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())?;
    let raw = std::fs::read_to_string(dir.join("manifest.json")).ok()?;
    let manifest: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let sq = manifest.get("squire")?;
    let file = sq.get("file")?.as_str()?.to_string();
    // Only offer it if the file is actually present to download.
    if !dir.join(&file).is_file() {
        return None;
    }

    let (host, port) = advertised_addr();
    let url = format!("http://{host}:{port}/app/{file}");
    let qr_svg = QrCode::new(url.as_bytes())
        .map(|c| {
            c.render::<svg::Color>()
                .min_dimensions(220, 220)
                .dark_color(svg::Color("#1c1b1f"))
                .light_color(svg::Color("#ffffff"))
                .build()
        })
        .ok();

    Some(AppInstallView {
        available: true,
        version_name: sq
            .get("versionName")
            .and_then(|v| v.as_str())
            .map(String::from),
        version_code: sq.get("versionCode").and_then(|v| v.as_u64()),
        file: Some(file),
        url: Some(url),
        qr_svg,
    })
}
