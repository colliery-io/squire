//! The Keep binary: wire a local single-tenant [`KeepState`] and serve the embedded admin UI on
//! **loopback only** (ADR SQUIRE-A-0008). Configuration comes from the environment with
//! sensible local defaults, so `cargo run -p keep` is the whole app.
//!
//! | Env var          | Default              | Meaning                                            |
//! |------------------|----------------------|----------------------------------------------------|
//! | `KEEP_DATA_DIR`  | `./squire-data`      | Directory holding the tenant's SQLite file         |
//! | `KEEP_HOUSEHOLD` | `keep`               | The local household handle (tenant)                |
//! | `KEEP_PORT`      | `4920`               | Loopback port for the admin UI                     |
//! | `KEEP_SIGNING_KEY` | dev placeholder    | HMAC key for operator tokens (set in production)   |

use std::path::PathBuf;

use domain_core::contract::HouseholdHandle;
use keep::{serve, KeepState};
use store::tenant::Backend;
use identity::TokenSigner;

/// One hour, in milliseconds — the operator token lifetime.
const TOKEN_TTL_MS: i64 = 60 * 60 * 1000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let data_dir =
        std::env::var("KEEP_DATA_DIR").unwrap_or_else(|_| "./squire-data".to_string());
    let household =
        std::env::var("KEEP_HOUSEHOLD").unwrap_or_else(|_| "keep".to_string());
    let port: u16 = std::env::var("KEEP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4920);
    let signing_key = std::env::var("KEEP_SIGNING_KEY")
        .unwrap_or_else(|_| "dev-keep-signing-key-change-me".to_string());

    let backend = Backend::Sqlite { dir: PathBuf::from(data_dir) };
    let state = KeepState::local(
        backend,
        HouseholdHandle(household),
        TokenSigner::new(signing_key.as_bytes()),
        TOKEN_TTL_MS,
    )?;

    eprintln!("the Keep is serving on http://127.0.0.1:{port} (loopback only)");
    serve(state, port).await
}
