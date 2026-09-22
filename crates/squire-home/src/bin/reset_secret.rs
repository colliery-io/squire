//! One-off operator tool: reset a member's login secret (the "I forgot my password" escape hatch).
//!
//! Usage:
//!   RESET_USER=<user id | display name> NEW_SECRET=<secret> [SQUIRE_HOUSEHOLD=home] reset_secret
//!   reset_secret                     # no RESET_USER → list the household's members and exit
//!
//! Unlike `add_admin` this keeps the member's **identity**: same `UserId`, so their paired devices,
//! history and audit trail are untouched — only the credential hash is replaced. Squire has no
//! in-app "change my secret", so this is the only recovery path; it is deliberately NOT a network
//! operation. Whoever can run it already owns the data dir (and `signing.key`), which is strictly
//! more power than a login, so it adds no new authority.
//!
//! Opens the persistent data dir (same defaulting as `squire-serve`). Run while the server is
//! stopped (single SQLite writer); in the container stack use `angreal deploy reset-secret`.

use std::path::PathBuf;

use domain_core::contract::{HouseholdHandle, Repository, UserId};
use identity::creds::{hash_secret, MIN_SECRET_LEN};
use squire_home::{env_nonempty, open_household, signing_key, TOKEN_TTL_MS};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data_dir: PathBuf = std::env::var_os("SQUIRE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("squire")
        });
    let handle = HouseholdHandle(env_nonempty("SQUIRE_HOUSEHOLD").unwrap_or_else(|| "home".into()));

    let key = signing_key(&data_dir)?;
    let (store, _identity) = open_household(&data_dir, &handle, &key, TOKEN_TTL_MS)?;
    let store = store.lock().expect("store mutex poisoned");
    let users = store.snapshot().users;

    let Some(wanted) = env_nonempty("RESET_USER") else {
        println!("members of household '{}':", handle.0);
        for u in &users {
            println!(
                "  {:>4}  {:<7} {}{}",
                u.id.0,
                format!("{:?}", u.role),
                u.display_name,
                if u.active { "" } else { "  (inactive)" }
            );
        }
        println!("set RESET_USER to an id or a display name (and NEW_SECRET) to reset one.");
        return Ok(());
    };

    // An id, else a display name that matches exactly one member (case-insensitive) — the same
    // resolution the Keep's login form uses, so "the name I sign in with" just works.
    let target = match wanted.parse::<u128>() {
        Ok(id) => users.iter().find(|u| u.id == UserId(id)),
        Err(_) => {
            let mut hits = users
                .iter()
                .filter(|u| u.display_name.eq_ignore_ascii_case(&wanted));
            match (hits.next(), hits.next()) {
                (Some(u), None) => Some(u),
                (Some(_), Some(_)) => {
                    return Err(format!("'{wanted}' matches several members — use the id").into())
                }
                _ => None,
            }
        }
    }
    .ok_or_else(|| format!("no member '{wanted}' in household '{}'", handle.0))?;

    // `env_nonempty`: a blank NEW_SECRET must never become an empty password (SQUIRE-T-0129).
    let secret = env_nonempty("NEW_SECRET").ok_or("set NEW_SECRET (non-empty)")?;
    if secret.chars().count() < MIN_SECRET_LEN {
        return Err(format!("NEW_SECRET must be at least {MIN_SECRET_LEN} characters").into());
    }

    store
        .set_credential(target.id, &hash_secret(&secret))
        .map_err(|e| format!("could not write the credential: {e:?}"))?;

    println!(
        "OK — reset the secret for {:?} '{}' (UserId {}) in household '{}'{}",
        target.role,
        target.display_name,
        target.id.0,
        handle.0,
        if target.active {
            ""
        } else {
            " — NOTE: this member is INACTIVE and still cannot log in"
        }
    );
    Ok(())
}
