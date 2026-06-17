//! `squire-serve` — the **persistent** home server (SQUIRE-T-0048).
//!
//! Unlike the `squire-home` demo (which wipes & re-seeds `/tmp` each run), this keeps data across
//! restarts in a durable OS data dir and does **register-or-load**: on first run it bootstraps the
//! first admin Knight from env; afterwards it opens the existing household **untouched** (no wipe,
//! no demo seed). The signing key is generated once and persisted, so previously paired devices keep
//! verifying after a restart.
//!
//! ## Configuration (env)
//! * `SQUIRE_DATA_DIR`   — data dir (default: `<OS data dir>/squire`, e.g. `~/.local/share/squire`).
//! * `SQUIRE_HOUSEHOLD`  — tenant handle (default `home`).
//! * `SQUIRE_ADMIN_NAME` / `SQUIRE_ADMIN_SECRET` — first-run admin (secret REQUIRED on first run).
//! * `API_PORT` (8080) / `KEEP_PORT` (4920) — listeners.
//! * `SQUIRE_SIGNING_KEY` — override the persisted key (raw bytes); `SQUIRE_MDNS=off` — disable mDNS.

use std::path::PathBuf;

use domain_core::contract::{HouseholdHandle, RegisterHouseholdReq};

use squire_home::{env_u16, has_admin, open_household, serve, signing_key, TOKEN_TTL_MS};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data_dir: PathBuf = std::env::var_os("SQUIRE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("squire")
        });
    let api_port = env_u16("API_PORT", 8080);
    let keep_port = env_u16("KEEP_PORT", 4920);
    let handle = HouseholdHandle(
        std::env::var("SQUIRE_HOUSEHOLD").unwrap_or_else(|_| "home".to_string()),
    );

    // Stable signing key (generated + persisted on first run) — paired tokens survive restarts.
    let key = signing_key(&data_dir)?;

    // Provision-if-absent + open (NEVER wiped). Idempotent on an existing store.
    let (store, identity) = open_household(&data_dir, &handle, &key, TOKEN_TTL_MS)?;

    // Register-or-load: bootstrap the first admin only if the household has none yet.
    let first_run = !has_admin(&store);
    if first_run {
        let admin_name = std::env::var("SQUIRE_ADMIN_NAME").unwrap_or_else(|_| "Admin".to_string());
        let admin_secret = std::env::var("SQUIRE_ADMIN_SECRET").map_err(|_| {
            format!(
                "first run for household '{}': set SQUIRE_ADMIN_SECRET (and optionally \
                 SQUIRE_ADMIN_NAME) to bootstrap the admin Knight",
                handle.0
            )
        })?;
        let resp = identity
            .register(RegisterHouseholdReq {
                household_name: handle.0.clone(),
                admin_name: admin_name.clone(),
                admin_secret,
            })
            .map_err(|e| format!("failed to bootstrap admin: {e:?}"))?;
        println!("  Bootstrapped household '{}' with admin '{admin_name}' (UserId {}).", handle.0, resp.admin.0);
    }

    println!("════════════════════════════════════════════════════════════════════");
    println!("  Squire server (squire-serve) — persistent, data in {}", data_dir.display());
    println!("  Household:               {}{}", handle.0, if first_run { " (newly bootstrapped)" } else { " (loaded)" });
    println!("  Keep (parent, loopback): http://127.0.0.1:{keep_port}");
    println!("  LAN api (phones):        http://0.0.0.0:{api_port}");
    if first_run {
        println!("  Next: open the Keep, sign in as the admin, add members, and pair devices.");
    }
    println!("════════════════════════════════════════════════════════════════════");

    serve(store, identity, handle, api_port, keep_port).await
}
