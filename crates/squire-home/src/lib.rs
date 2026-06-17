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

use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};

use api::AppState;
use keep::KeepState;

use domain_core::contract::{HouseholdHandle, Repository, Role};
use identity::{Identity, ProdIdentity, SharedStore, TokenSigner};
use store::tenant::{Backend, Provisioner};
use store::SystemClock;

/// Standard token lifetime: 24h.
pub const TOKEN_TTL_MS: i64 = 24 * 60 * 60 * 1000;

type BoxErr = Box<dyn std::error::Error + Send + Sync>;

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
    let app = AppState::new(store.clone(), identity.clone());
    let keep_state = KeepState::from_parts(store.clone(), identity.clone(), handle.clone());

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
