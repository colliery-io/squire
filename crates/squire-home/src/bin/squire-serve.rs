//! `squire-serve` — the **persistent** home server (SQUIRE-T-0048).
//!
//! Keeps data across restarts in a durable OS data dir and does **register-or-load**: on first run it
//! bootstraps the first admin Knight from env (or, with no `SQUIRE_ADMIN_SECRET`, starts admin-less so
//! the operator can create one in the Keep's first-run form); afterwards it opens the existing
//! household **untouched**. The signing key is generated once and persisted.
//!
//! The actual server logic lives in [`squire_home::run_home_server`], shared with the native desktop
//! app (SQUIRE-T-0092). This binary just self-updates, then runs it on a tokio runtime.
//!
//! ## Configuration (env)
//! * `SQUIRE_DATA_DIR`   — data dir (default: `<OS data dir>/squire`).
//! * `SQUIRE_HOUSEHOLD`  — tenant handle (default `home`).
//! * `SQUIRE_ADMIN_NAME` / `SQUIRE_ADMIN_SECRET` — first-run admin (optional; else create it in the Keep).
//! * `API_PORT` (8080) / `KEEP_PORT` (4920) — listeners.
//! * `SQUIRE_APK_DIR` — OTA app-update dir (default: `<data_dir>/updates`, created on start).
//! * `SQUIRE_SIGNING_KEY` — override the persisted key; `SQUIRE_MDNS=off` — disable mDNS;
//!   `SQUIRE_SELF_UPDATE=off` — disable self-update.

use squire_home::{init_tracing, maybe_self_update, run_home_server};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Install the tracing subscriber first so the self-update step (and everything after) logs
    // through it (SQUIRE-T-0127). `RUST_LOG` controls the level (default `info`).
    init_tracing();
    // Self-update BEFORE the async runtime: `self_update` uses blocking HTTP, and nesting a runtime
    // inside tokio panics. Re-execs into the new binary if it updated (does not return); no-op for a
    // `cargo run` dev build or when SQUIRE_SELF_UPDATE=off.
    maybe_self_update();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run_home_server())
}
