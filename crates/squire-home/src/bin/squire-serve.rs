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
//! * `SQUIRE_APK_DIR` — OTA app-update dir (default: `<data_dir>/updates`, created on start).
//! * `SQUIRE_SIGNING_KEY` — override the persisted key (raw bytes); `SQUIRE_MDNS=off` — disable mDNS.

use std::path::PathBuf;

use domain_core::contract::{HouseholdHandle, RegisterHouseholdReq};

use squire_home::{
    ensure_timezone, env_u16, has_admin, local_lan_ip, maybe_self_update, open_household, serve,
    signing_key, spawn_apk_sync, TOKEN_TTL_MS,
};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Self-update BEFORE the async runtime: `self_update` uses blocking HTTP, and nesting a runtime
    // inside tokio panics. Re-execs into the new binary if it updated (does not return); no-op for a
    // `cargo run` dev build or when SQUIRE_SELF_UPDATE=off.
    maybe_self_update();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(serve_main())
}

async fn serve_main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data_dir: PathBuf = std::env::var_os("SQUIRE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("squire")
        });

    // OTA app-update dir (SQUIRE-T-0051/0085): default to `<data_dir>/updates` and create it, so the
    // `/app/*` endpoints are always available without setting `SQUIRE_APK_DIR`. An explicit env var
    // still wins. Dropping a new APK + manifest there is picked up live (read per request) — no
    // relaunch needed.
    let apk_dir: PathBuf = std::env::var_os("SQUIRE_APK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("updates"));
    let _ = std::fs::create_dir_all(&apk_dir);
    std::env::set_var("SQUIRE_APK_DIR", &apk_dir);

    let api_port = env_u16("API_PORT", 8080);
    let keep_port = env_u16("KEEP_PORT", 4920);
    let handle = HouseholdHandle(
        std::env::var("SQUIRE_HOUSEHOLD").unwrap_or_else(|_| "home".to_string()),
    );

    // Stable signing key (generated + persisted on first run) — paired tokens survive restarts.
    let key = signing_key(&data_dir)?;

    // Provision-if-absent + open (NEVER wiped). Idempotent on an existing store.
    let (store, identity) = open_household(&data_dir, &handle, &key, TOKEN_TTL_MS)?;

    // Onboarding (ADR A-0011): ensure a household timezone is set — seeded from SQUIRE_TZ or the
    // detected host zone on first run, respected thereafter — so "midnight" is the family's midnight.
    let timezone = ensure_timezone(&store);

    // Register-or-load: bootstrap the first admin only if the household has none yet. With
    // `SQUIRE_ADMIN_SECRET` set we bootstrap headlessly (scripts / the old flow); WITHOUT it we start
    // anyway and let the operator create the admin in the browser via the Keep's first-run register
    // form (SQUIRE-T-0091) — so a double-clickable launch needs no terminal or env vars.
    let first_run = !has_admin(&store);
    if first_run {
        match std::env::var("SQUIRE_ADMIN_SECRET") {
            Ok(admin_secret) => {
                let admin_name =
                    std::env::var("SQUIRE_ADMIN_NAME").unwrap_or_else(|_| "Admin".to_string());
                let resp = identity
                    .register(RegisterHouseholdReq {
                        household_name: handle.0.clone(),
                        admin_name: admin_name.clone(),
                        admin_secret,
                    })
                    .map_err(|e| format!("failed to bootstrap admin: {e:?}"))?;
                println!(
                    "  Bootstrapped household '{}' with admin '{admin_name}' (UserId {}).",
                    handle.0, resp.admin.0
                );
            }
            Err(_) => {
                println!(
                    "  First run — no admin yet. Open the Keep below and use \
                     \"First run? Create the admin Knight\" to set up your account."
                );
            }
        }
    }

    // Advertise the real LAN IP in the pairing QR (the Keep reads `SQUIRE_PAIR_HOST`), so phones
    // reach THIS machine — not the emulator-only `10.0.2.2`. Honour an explicit override.
    let lan_host = std::env::var("SQUIRE_PAIR_HOST").ok().or_else(|| {
        let detected = local_lan_ip().map(|ip| ip.to_string());
        if let Some(ip) = &detected {
            std::env::set_var("SQUIRE_PAIR_HOST", ip);
        }
        detected
    });

    println!("════════════════════════════════════════════════════════════════════");
    println!("  Squire server (squire-serve) — persistent, data in {}", data_dir.display());
    println!("  Household:               {}{}", handle.0, if first_run { " (newly bootstrapped)" } else { " (loaded)" });
    println!("  Timezone:                {timezone}   ← daily quests reset at this local midnight");
    if let Some(apk) = std::env::var_os("SQUIRE_APK_DIR") {
        println!("  App updates (OTA):       {}   ← drop <name>.apk + manifest.json here (live, no relaunch)", PathBuf::from(apk).display());
    }
    println!("  Keep (parent, loopback): http://127.0.0.1:{keep_port}");
    match &lan_host {
        Some(h) => {
            println!("  LAN api (phones):        http://{h}:{api_port}   ← phones pair to this (QR + mDNS)");
        }
        None => {
            println!("  LAN api (phones):        http://0.0.0.0:{api_port}");
            println!("  (could not detect a LAN IP — set SQUIRE_PAIR_HOST=<this computer's IP> for the QR)");
        }
    }
    if first_run {
        println!("  Next: open the Keep, sign in as the admin, add members, and pair devices.");
    }
    println!("════════════════════════════════════════════════════════════════════");

    // Keep the LAN-served phone APK current from the public dist repo (SQUIRE-T-0087 / A-0012).
    // Background + best-effort: never blocks the serve loop or fails startup.
    spawn_apk_sync(apk_dir);

    serve(store, identity, handle, api_port, keep_port).await
}
