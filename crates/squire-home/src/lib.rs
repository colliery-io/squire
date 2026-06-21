//! Shared home-server wiring for the two binaries in this crate:
//!
//! * **`squire-home`** (`src/main.rs`) — the throwaway **demo** harness: a fixed `/tmp` dir wiped &
//!   re-seeded each run (a "demo" household with sample quests/rewards). For the emulator one-tap
//!   flow and manual demos.
//! * **`squire-serve`** (`src/bin/squire-serve.rs`) — the **persistent** server (SQUIRE-T-0048): a
//!   durable OS data dir, **register-or-load** (bootstrap the first admin only if absent, never
//!   wipe/seed), and a **stable signing key** so paired devices keep working across restarts.
//!
//! Both compose the same coherent topology (ADR A-0008): one shared single-writer store + identity,
//! the loopback **Keep** and the LAN **api** over it, plus a best-effort mDNS advert (T-0047).

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use api::AppState;
use keep::KeepState;

use domain_core::contract::{config_keys, HouseholdHandle, RegisterHouseholdReq, Repository, Role};
use identity::{Identity, ProdIdentity, SharedStore, TokenSigner};
use store::tenant::{Backend, Provisioner};
use store::SystemClock;

mod apk_sync;
pub use apk_sync::spawn_apk_sync;

mod updater;
pub use updater::maybe_self_update;

/// Default token lifetime: effectively forever (~100 years). Paired devices have no token-refresh
/// path and a Squire (kid) has no re-login, so a short TTL silently bricks the app daily (it just
/// reads "offline"); device tokens are revocable via unpair/deactivate, so a very long life is the
/// right model. Override with `SQUIRE_TOKEN_TTL_MS` (see `token_ttl_ms`).
pub const TOKEN_TTL_MS: i64 = 100 * 365 * 24 * 60 * 60 * 1000;

/// The effective token lifetime: `SQUIRE_TOKEN_TTL_MS` if set + valid, else [`TOKEN_TTL_MS`].
pub fn token_ttl_ms() -> i64 {
    std::env::var("SQUIRE_TOKEN_TTL_MS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(TOKEN_TTL_MS)
}

type BoxErr = Box<dyn std::error::Error + Send + Sync>;

/// Run the persistent home server end to end: resolve the data + OTA dirs, open (provision-if-absent)
/// the household, seed the timezone, optionally bootstrap the admin, advertise the LAN address, start
/// the background APK sync, and serve the api + Keep until shut down. Shared by the headless
/// `squire-serve` binary and the native desktop app (SQUIRE-T-0092) so both run an identical server.
pub async fn run_home_server() -> Result<(), BoxErr> {
    let data_dir: PathBuf = std::env::var_os("SQUIRE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("squire"));

    // OTA app-update dir (SQUIRE-T-0051/0085): default to `<data_dir>/updates` and create it, so the
    // `/app/*` endpoints are always available without setting `SQUIRE_APK_DIR`. An explicit env var
    // still wins. Dropping a new APK + manifest there is picked up live (read per request).
    let apk_dir: PathBuf = std::env::var_os("SQUIRE_APK_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("updates"));
    let _ = std::fs::create_dir_all(&apk_dir);
    std::env::set_var("SQUIRE_APK_DIR", &apk_dir);

    let api_port = env_u16("API_PORT", 8080);
    let keep_port = env_u16("KEEP_PORT", 4920);
    let handle =
        HouseholdHandle(std::env::var("SQUIRE_HOUSEHOLD").unwrap_or_else(|_| "home".to_string()));

    // Stable signing key (generated + persisted on first run) — paired tokens survive restarts.
    let key = signing_key(&data_dir)?;

    // Provision-if-absent + open (NEVER wiped). Idempotent on an existing store.
    let (store, identity) = open_household(&data_dir, &handle, &key, token_ttl_ms())?;

    // Onboarding (ADR A-0011): ensure a household timezone is set.
    let timezone = ensure_timezone(&store);

    // Register-or-load: bootstrap the first admin only if none yet. With `SQUIRE_ADMIN_SECRET` set we
    // bootstrap headlessly; without it we start anyway and the operator creates the admin in the
    // browser via the Keep's first-run register form (SQUIRE-T-0091).
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
                    "  First run — no admin yet. Open the Keep and use \"First run? Create the \
                     admin Knight\" to set up your account."
                );
            }
        }
    }

    // Advertise the real LAN IP in the pairing QR (the Keep reads `SQUIRE_PAIR_HOST`).
    let lan_host = std::env::var("SQUIRE_PAIR_HOST").ok().or_else(|| {
        let detected = local_lan_ip().map(|ip| ip.to_string());
        if let Some(ip) = &detected {
            std::env::set_var("SQUIRE_PAIR_HOST", ip);
        }
        detected
    });

    println!("════════════════════════════════════════════════════════════════════");
    println!("  Squire home server — persistent, data in {}", data_dir.display());
    println!(
        "  Household:               {}{}",
        handle.0,
        if first_run { " (newly bootstrapped)" } else { " (loaded)" }
    );
    println!("  Timezone:                {timezone}   ← daily quests reset at this local midnight");
    if let Some(apk) = std::env::var_os("SQUIRE_APK_DIR") {
        println!("  App updates (OTA):       {}", PathBuf::from(apk).display());
    }
    println!("  Keep (parent, loopback): http://127.0.0.1:{keep_port}");
    match &lan_host {
        Some(h) => println!("  LAN api (phones):        http://{h}:{api_port}   ← phones pair here (QR + mDNS)"),
        None => {
            println!("  LAN api (phones):        http://0.0.0.0:{api_port}");
            println!("  (could not detect a LAN IP — set SQUIRE_PAIR_HOST=<this computer's IP>)");
        }
    }
    println!("════════════════════════════════════════════════════════════════════");

    // Keep the LAN-served phone APK current from the public dist repo (SQUIRE-T-0087 / A-0012).
    spawn_apk_sync(apk_dir);

    serve(store, identity, handle, api_port, keep_port).await
}

/// **Provision-if-absent** + open the tenant store on a SQLite dir, and build the shared
/// single-writer [`ProdIdentity`] over it. Provisioning is idempotent (migrations skip already-
/// applied), so calling this on an existing data dir opens it **untouched** — no wipe, no reseed.
pub fn open_household(
    dir: &Path,
    handle: &HouseholdHandle,
    signing_key: &[u8],
    token_ttl_ms: i64,
) -> Result<(SharedStore, Arc<dyn Identity>), BoxErr> {
    let provisioner = Provisioner::new(Backend::Sqlite { dir: dir.to_path_buf() });
    provisioner.provision(&handle.0)?;
    let store: SharedStore = Arc::new(Mutex::new(provisioner.open(&handle.0, SystemClock)?));
    let identity: Arc<dyn Identity> = Arc::new(ProdIdentity::shared_local(
        store.clone(),
        TokenSigner::new(signing_key),
        handle.clone(),
        token_ttl_ms,
    ));
    Ok((store, identity))
}

/// Ensure the household has a timezone setting (ADR SQUIRE-A-0011 onboarding, SQUIRE-T-0067).
///
/// **Idempotent**: an existing, non-empty `timezone` is returned untouched (so a parent's later
/// choice via the Keep is never clobbered). If absent, seed it from `SQUIRE_TZ` (when it's a valid
/// IANA zone), else the **auto-detected host zone** (`iana-time-zone`), else `"UTC"`; the candidate
/// is validated (`store::valid_timezone`) before persisting, so the stored value is always real.
/// Returns the active zone name. Call before [`serve`] so the clock is built from the seeded value.
pub fn ensure_timezone(store: &SharedStore) -> String {
    let store = store.lock().expect("store mutex poisoned");
    if let Some(tz) = store.get_setting(config_keys::TIMEZONE) {
        if !tz.is_empty() {
            return tz;
        }
    }
    let candidate = std::env::var("SQUIRE_TZ")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| iana_time_zone::get_timezone().ok())
        .unwrap_or_else(|| "UTC".to_string());
    let zone = if store::valid_timezone(&candidate) {
        candidate
    } else {
        "UTC".to_string()
    };
    store
        .set_setting(config_keys::TIMEZONE, &zone, None)
        .expect("seed timezone setting");
    zone
}

/// Whether the household already has an admin Knight — i.e. it's been bootstrapped (so the
/// persistent server should *load* it rather than register a new admin).
pub fn has_admin(store: &SharedStore) -> bool {
    store
        .lock()
        .expect("store mutex poisoned")
        .snapshot()
        .users
        .iter()
        .any(|u| u.role == Role::Knight)
}

/// Serve **both surfaces** over the one shared store/identity: the LAN api on `0.0.0.0:api_port`
/// (phones) and the loopback Keep on `keep_port` (the parent's admin UI), plus a best-effort mDNS
/// advert. Returns only if a listener fails.
pub async fn serve(
    store: SharedStore,
    identity: Arc<dyn Identity>,
    handle: HouseholdHandle,
    api_port: u16,
    keep_port: u16,
) -> Result<(), BoxErr> {
    // One shared household-local clock (ADR A-0011) seeded from the store's config, so the api and
    // the Keep read — and the Keep hot-swaps — the SAME live timezone cell (a Settings change
    // applies to both surfaces without a restart).
    let clock = {
        let cfg = store.lock().expect("store mutex poisoned").load_config();
        store::LocalClock::new(store::live_config(cfg))
    };
    let app = AppState::with_clock(store.clone(), identity.clone(), clock.clone());
    let keep_state = KeepState::from_parts_with_clock(store.clone(), identity.clone(), handle.clone(), clock);

    // Held for the process lifetime so the advertisement persists.
    let _mdns = start_mdns(api_port, &handle.0);

    tokio::try_join!(
        api::serve(app, SocketAddr::from(([0, 0, 0, 0], api_port))),
        keep::serve(keep_state, keep_port),
    )?;
    Ok(())
}

/// Parse a `u16` from env `key`, falling back to `default`.
pub fn env_u16(key: &str, default: u16) -> u16 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Best-effort detection of this machine's **LAN IPv4** — the address a phone on the same Wi-Fi
/// should reach the api at (SQUIRE-T-0050). "Connects" a UDP socket toward a public address so the
/// OS picks the outbound interface, then reads its local IP; **no packets are sent**. Returns `None`
/// (e.g. no network) so the caller can fall back. Skips loopback / non-IPv4.
pub fn local_lan_ip() -> Option<IpAddr> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    // The destination is never contacted; this only selects a route/interface.
    sock.connect("8.8.8.8:80").ok()?;
    let ip = sock.local_addr().ok()?.ip();
    if ip.is_loopback() || !ip.is_ipv4() {
        None
    } else {
        Some(ip)
    }
}

/// The server's HMAC signing key, **stable across restarts** so previously issued/paired tokens keep
/// verifying (SQUIRE-T-0048). Resolution: `SQUIRE_SIGNING_KEY` env (raw bytes) if set & non-empty;
/// otherwise read `dir/signing.key`, generating + persisting 32 random bytes (mode `0600` on unix)
/// on first run.
pub fn signing_key(dir: &Path) -> std::io::Result<Vec<u8>> {
    if let Ok(env_key) = std::env::var("SQUIRE_SIGNING_KEY") {
        if !env_key.is_empty() {
            return Ok(env_key.into_bytes());
        }
    }
    let path = dir.join("signing.key");
    if let Ok(bytes) = std::fs::read(&path) {
        if !bytes.is_empty() {
            return Ok(bytes);
        }
    }
    std::fs::create_dir_all(dir)?;
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    write_secret(&path, &key)?;
    Ok(key.to_vec())
}

/// Write `bytes` to `path`, restricting to owner-read/write (`0600`) on unix.
fn write_secret(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        f.write_all(bytes)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, bytes)
    }
}

/// Advertise the LAN api over mDNS/DNS-SD as `_squire._tcp` on `api_port` (SQUIRE-T-0047 / NFR-6),
/// so a pairing phone's NSD browse can prefill the host/port. **Best-effort**: opt out with
/// `SQUIRE_MDNS=off`, and a responder failure only logs — serving never depends on it. The returned
/// guard must be kept alive for the advertisement to persist. `libmdns` enumerates the host's
/// interfaces and announces the machine's LAN address (not loopback) itself.
pub fn start_mdns(api_port: u16, household: &str) -> Option<(libmdns::Responder, libmdns::Service)> {
    if std::env::var("SQUIRE_MDNS").is_ok_and(|v| v.eq_ignore_ascii_case("off")) {
        println!("  mDNS:                    disabled (SQUIRE_MDNS=off)");
        return None;
    }
    match libmdns::Responder::new() {
        Ok(responder) => {
            let txt = format!("household={household}");
            let service = responder.register(
                "_squire._tcp".to_owned(),
                "Squire".to_owned(),
                api_port,
                &[txt.as_str()],
            );
            println!("  mDNS:                    advertising _squire._tcp on :{api_port}");
            Some((responder, service))
        }
        Err(e) => {
            eprintln!("  mDNS:                    disabled (responder failed: {e})");
            None
        }
    }
}
