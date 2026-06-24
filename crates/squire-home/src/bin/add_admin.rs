//! One-off operator tool: insert a Knight admin into an existing household.
//!
//! Usage:
//!   ADMIN_NAME=Dad ADMIN_SECRET=pass [ACTING_KNIGHT=2] [SQUIRE_HOUSEHOLD=home] \
//!     cargo run -p squire-home --bin add_admin
//!
//! Opens the persistent data dir (same defaulting as `squire-serve`), then calls the identity
//! component's Knight-only `add_member` — authorised by an existing Knight in the household — to
//! mint a new Knight with a hashed secret. Run while the server is stopped (single SQLite writer).

use std::path::PathBuf;

use domain_core::contract::{AddMemberReq, HouseholdHandle, Role, UserId};
use identity::Principal;
use squire_home::{open_household, signing_key, TOKEN_TTL_MS};

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data_dir: PathBuf = std::env::var_os("SQUIRE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("squire")
        });
    let handle =
        HouseholdHandle(std::env::var("SQUIRE_HOUSEHOLD").unwrap_or_else(|_| "home".into()));
    let name = std::env::var("ADMIN_NAME").unwrap_or_else(|_| "Dad".into());
    let secret = std::env::var("ADMIN_SECRET").expect("set ADMIN_SECRET");
    // An existing Knight to authorise the add (defense-in-depth requires a Knight caller).
    let acting: u128 = std::env::var("ACTING_KNIGHT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2);

    let key = signing_key(&data_dir)?;
    let (_store, identity) = open_household(&data_dir, &handle, &key, TOKEN_TTL_MS)?;

    let caller = Principal {
        household: handle.clone(),
        user: UserId(acting),
        role: Role::Knight,
    };
    let resp = identity
        .add_member(
            &caller,
            AddMemberReq {
                role: Role::Knight,
                display_name: name.clone(),
                initial_secret: secret,
            },
        )
        .map_err(|e| format!("add_member failed ({e:?}) — is ACTING_KNIGHT a real Knight id?"))?;

    println!(
        "OK — added Knight '{name}' as UserId {} in household '{}'",
        resp.user.0, handle.0
    );
    Ok(())
}
